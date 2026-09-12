"""Cross-check the Rust release verifier with independent OpenSSL signatures.

All identities, bytes and keys here are disposable synthetic fixtures. This is
not an OS release and cannot produce a production signing key or promotion.
"""
import argparse
import base64
import hashlib
import json
import pathlib
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("--workdir", type=pathlib.Path, required=True)
parser.add_argument("--sysroot", type=pathlib.Path)
parser.add_argument("--openssl", default="openssl")
args = parser.parse_args()
root = args.workdir.resolve()
root.mkdir(parents=True, exist_ok=False)
binary = args.sysroot.resolve() if args.sysroot else None
private = root / "disposable.pem"
wrong_private = root / "wrong-disposable.pem"
public = root / "release.pub"
wrong_public = root / "wrong.pub"
payload = root / "release.json"
artifact = root / "kedra-desktop-44-42.iso"
artifact.write_bytes(b"abc")
release = {
    "schema_version": 1,
    "project": "Kedra",
    "scope": {"target": "desktop", "architecture": "x86_64", "fedora_release": 44, "repository": "ghcr.io/reidond/kedra-desktop"},
    "sequence": 42,
    "source_revision": "a" * 40,
    "build": {"repository": "Reidond/kedra", "workflow": ".github/workflows/release.yml", "run_id": 123, "run_attempt": 1},
    "image_digest": "sha256:" + "b" * 64,
    "home_manifest_sha256": "c" * 64,
    "installer": {"filename": artifact.name, "size_bytes": 3, "sha256": hashlib.sha256(b"abc").hexdigest()},
    "approval": "promoted",
    "minimum_protocol": 1,
}
payload.write_text(json.dumps(release, separators=(",", ":")), encoding="utf-8")
def openssl(*arguments):
    return subprocess.run([args.openssl, *map(str, arguments)], check=True, capture_output=True).stdout

def verify(signature, key=public, manifest=payload, image=artifact, expected=True):
    result = subprocess.run([str(binary), "release", "verify", "--manifest", str(manifest),
        "--signature", str(signature), "--public-key", str(key), "--artifact", str(image),
        "--target", "desktop", "--json"], capture_output=True)
    if expected:
        if result.returncode != 0:
            raise RuntimeError("Rust rejected an OpenSSL signature: " + result.stderr.decode())
        report = json.loads(result.stdout)
        assert report["signature_valid"] and report["artifact_verified"]
        assert not report["deployment_authorized"] and not report["channel_freshness_verified"]
    else:
        assert result.returncode != 0 and not result.stdout, "Invalid input received success output"

try:
    print(openssl("version").decode().strip(), flush=True)
    for secret, pub in [(private, public), (wrong_private, wrong_public)]:
        openssl("genpkey", "-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:P-256", "-out", secret)
        secret.chmod(0o600)
        openssl("pkey", "-in", secret, "-pubout", "-out", pub)
    for number in range(16):
        # OpenSSL chooses independent nonces, independently of RustCrypto.
        der = openssl("dgst", "-sha256", "-sign", private, payload)
        signature = root / f"signature-{number}.sig"
        signature.write_bytes(base64.b64encode(der) + b"\n")
        if binary:
            verify(signature)
    if binary:
        identity = subprocess.run([str(binary), 'release', 'key', '--public-key', str(public), '--json'], capture_output=True, check=True)
        identity = json.loads(identity.stdout)
        expected = hashlib.sha256(openssl('pkey', '-pubin', '-in', public, '-outform', 'DER')).hexdigest()
        assert identity['key_fingerprint_sha256'] == expected and not identity['trust_established']
        invalid_key = subprocess.run([str(binary), 'release', 'key', '--public-key', str(private)], capture_output=True)
        assert invalid_key.returncode != 0 and not invalid_key.stdout
        # Public release tooling checks freshness using independently signed
        # channel bytes. Only the actual CLI supplies retained state.
        now = int(time.time())
        checkpoint = root / 'checkpoint.json'
        checkpoint_signature = root / 'checkpoint.sig'
        channel_record = {'schema_version': 1, 'project': 'Kedra', 'scope': release['scope'],
                          'generation': 1, 'release_sha256': hashlib.sha256(payload.read_bytes()).hexdigest(),
                          'issued_at': now, 'expires_at': now + 3600, 'last_successful_resolution': now}
        channel_command = [str(binary), 'release', 'channel', '--manifest', str(payload),
                           '--signature', str(signature), '--checkpoint', str(checkpoint),
                           '--checkpoint-signature', str(checkpoint_signature), '--public-key', str(public),
                           '--target', 'desktop', '--repository', release['scope']['repository'], '--json']

        def channel(record, previous=None, expected=True):
            checkpoint.write_text(json.dumps(record, separators=(',', ':')), encoding='utf-8')
            checkpoint_signature.write_bytes(base64.b64encode(openssl('dgst', '-sha256', '-sign', private, checkpoint)))
            arguments = channel_command + (['--previous-state', str(previous)] if previous else [])
            result = subprocess.run(arguments, capture_output=True)
            if expected:
                if result.returncode != 0:
                    raise RuntimeError('CLI channel verification failed: ' + result.stderr.decode())
                value = json.loads(result.stdout)
                assert value['channel_freshness_verified'] and not value['deployment_authorized']
                assert value['replay_checked'] == (previous is not None)
                return value
            assert result.returncode != 0 and not result.stdout, 'Invalid channel received success output'

        first = channel(channel_record)
        prior = root / 'prior-channel-state.json'
        prior.write_text(json.dumps(first['next_trust_state']), encoding='utf-8')
        prior_bytes = prior.read_bytes()
        channel(channel_record, prior)
        channel({**channel_record, 'expires_at': now + 3599}, prior, expected=False)
        channel({**channel_record, 'issued_at': now - 120, 'expires_at': now - 60,
                 'last_successful_resolution': now - 120}, expected=False)
        channel({**channel_record, 'issued_at': now + 3600, 'expires_at': now + 7200}, expected=False)
        channel({**channel_record, 'release_sha256': 'f' * 64}, expected=False)
        renewed = channel({**channel_record, 'generation': 2}, prior)
        newer = root / 'newer-channel-state.json'
        newer.write_text(json.dumps(renewed['next_trust_state']), encoding='utf-8')
        channel(channel_record, newer, expected=False)
        assert prior.read_bytes() == prior_bytes, 'Read-only channel verification changed retained state'
        print('PASS: CLI signed channel initial/repeat/no-change checks and expiry/future/binding/replay refusal', flush=True)
        # Authenticate an explicitly historical predecessor through the public
        # CLI, then use its returned ordering floor for a fresh signed pair.
        history_command = channel_command.copy()
        history_command[2] = 'history'

        def history(record, previous=None, expected=True, expired=True,
                    manifest=payload, manifest_signature=signature, key=public,
                    signer=private, raw=None):
            checkpoint.write_bytes(raw if raw is not None else json.dumps(record, separators=(',', ':')).encode())
            checkpoint_signature.write_bytes(base64.b64encode(openssl('dgst', '-sha256', '-sign', signer, checkpoint)))
            arguments = history_command.copy()
            for option, value in [('--manifest', manifest), ('--signature', manifest_signature), ('--public-key', key)]:
                arguments[arguments.index(option) + 1] = str(value)
            if previous is not None:
                arguments += ['--previous-state', str(previous)]
            started = int(time.time())
            result = subprocess.run(arguments, capture_output=True)
            if not expected:
                assert result.returncode != 0 and not result.stdout, 'Invalid predecessor received history output'
                return None
            if result.returncode:
                raise RuntimeError('CLI history verification failed: ' + result.stderr.decode())
            value = json.loads(result.stdout)
            assert value['signature_valid'] and value['historical_only']
            assert not value['channel_freshness_verified'] and not value['deployment_authorized']
            assert 'next_trust_state' not in value and value['expired'] == expired
            assert value['replay_checked'] == (previous is not None)
            assert started <= value['verified_at'] <= int(time.time()), 'History did not use the actual clock'
            return value

        expired_record = {**channel_record, 'generation': 8, 'issued_at': now - 120,
                          'expires_at': now - 60, 'last_successful_resolution': now - 120}
        historical = history(expired_record)
        ordering = root / 'historical-ordering.json'
        ordering.write_text(json.dumps(historical['ordering_state']), encoding='utf-8')
        ordering_bytes = ordering.read_bytes()
        history(expired_record, ordering)
        history(channel_record, expired=False)
        channel(expired_record, expected=False)
        # The strict unpack interface must also keep refusing this same expired pair.
        expired_bundle = root / 'expired-channel.json'
        expired_bundle.write_text(json.dumps({'schema_version': 1,
            'release': {'payload': payload.read_text(), 'signature': signature.read_text()},
            'checkpoint': {'payload': checkpoint.read_text(), 'signature': checkpoint_signature.read_text()}}), encoding='utf-8')
        expired_output = root / 'expired-unpack'
        result = subprocess.run([str(binary), 'release', 'unpack', '--bundle', str(expired_bundle),
            '--public-key', str(public), '--expected-fingerprint', expected, '--target', 'desktop',
            '--repository', release['scope']['repository'], '--output-dir', str(expired_output)], capture_output=True)
        assert result.returncode != 0 and not expired_output.exists(), 'Historical metadata became eligible for unpack'

        history(expired_record, key=wrong_public, expected=False)
        history(expired_record, signer=wrong_private, expected=False)
        changed_release = root / 'historical-altered-release.json'
        changed_release.write_bytes(payload.read_bytes() + b' ')
        history(expired_record, manifest=changed_release, expected=False)
        for changed in [
            {'schema_version': 2}, {'unknown': True}, {'release_sha256': 'f' * 64},
            {'scope': {**release['scope'], 'target': 'other'}}, {'generation': 0},
            {'expires_at': now - 120},
            {'issued_at': now - 8 * 86400, 'last_successful_resolution': now - 8 * 86400},
            {'issued_at': now + 3600, 'expires_at': now + 7200, 'last_successful_resolution': now},
            {'last_successful_resolution': 0}, {'last_successful_resolution': now},
        ]:
            history({**expired_record, **changed}, expected=False)
        duplicate = json.dumps(expired_record, separators=(',', ':'))[:-1] + ',"generation":8}'
        history(expired_record, raw=duplicate.encode(), expected=False)
        history({**expired_record, 'generation': 7}, ordering, expected=False)
        history({**expired_record, 'expires_at': now - 59}, ordering, expected=False)
        history({**expired_record, 'generation': 9, 'last_successful_resolution': now - 121}, ordering, expected=False)
        for name, changes in [('older-release', {'sequence': 41}), ('changed-same-release', {'source_revision': 'd' * 40})]:
            other_manifest, other_signature = root / f'{name}.json', root / f'{name}.sig'
            other_manifest.write_text(json.dumps({**release, **changes}), encoding='utf-8')
            other_signature.write_bytes(base64.b64encode(openssl('dgst', '-sha256', '-sign', private, other_manifest)))
            other_checkpoint = {**expired_record, 'generation': 9,
                                'release_sha256': hashlib.sha256(other_manifest.read_bytes()).hexdigest()}
            history(other_checkpoint, ordering, expected=False, manifest=other_manifest, manifest_signature=other_signature)
        fresh_after_expiry = channel({**channel_record, 'generation': 9}, ordering)
        assert fresh_after_expiry['next_trust_state']['generation'] == 9
        assert fresh_after_expiry['next_trust_state']['highest_release_sequence'] == 42
        assert ordering.read_bytes() == ordering_bytes, 'Historical verification changed the retained ordering floor'
        print('PASS: CLI historical-only expired predecessor, fresh higher continuation, unchanged ordering and strict incoming expiry', flush=True)
        print('PASS: history rejects wrong signatures, malformed scope/schema/binding/lifetime/future and ordering regressions', flush=True)
        # Exercise downloaded channel -> verified files -> ordinary channel CLI.
        channel(channel_record)
        bundle = {'schema_version': 1,
                  'release': {'payload': payload.read_text(), 'signature': signature.read_text()},
                  'checkpoint': {'payload': checkpoint.read_text(), 'signature': checkpoint_signature.read_text()}}
        bundle_path = root / 'channel.json'
        bundle_path.write_text(json.dumps(bundle), encoding='utf-8')
        unpacked = root / 'unpacked'
        unpack = [str(binary), 'release', 'unpack', '--bundle', str(bundle_path),
                  '--public-key', str(public), '--expected-fingerprint', expected,
                  '--target', 'desktop', '--repository', release['scope']['repository'],
                  '--output-dir', str(unpacked), '--json']
        result = subprocess.run(unpack + ['--previous-state', str(newer)], capture_output=True)
        assert result.returncode != 0 and not unpacked.exists(), 'Replayed bundle created output'
        altered_bundle = json.loads(json.dumps(bundle))
        altered_bundle['release']['payload'] += ' '
        bundle_path.write_text(json.dumps(altered_bundle), encoding='utf-8')
        result = subprocess.run(unpack, capture_output=True)
        assert result.returncode != 0 and not unpacked.exists(), 'Tampered bundle created output'
        bundle_path.write_text(json.dumps(bundle), encoding='utf-8')
        wrong_fingerprint = unpack.copy()
        wrong_fingerprint[wrong_fingerprint.index('--expected-fingerprint') + 1] = '0' * 64
        result = subprocess.run(wrong_fingerprint, capture_output=True)
        assert result.returncode != 0 and not unpacked.exists(), 'Wrong authority created output'
        result = subprocess.run(unpack + ['--previous-state', str(prior)], capture_output=True)
        if result.returncode:
            raise RuntimeError('Channel unpack failed: ' + result.stderr.decode())
        assert json.loads(result.stdout)['replay_checked']
        for name, original in [('release.json', payload), ('release.sig', signature),
                               ('checkpoint.json', checkpoint), ('checkpoint.sig', checkpoint_signature)]:
            assert (unpacked / name).read_bytes() == original.read_bytes(), 'Unpack changed exact signed bytes'
        unpacked_channel = [str(binary), 'release', 'channel', '--manifest', str(unpacked / 'release.json'),
                            '--signature', str(unpacked / 'release.sig'), '--checkpoint', str(unpacked / 'checkpoint.json'),
                            '--checkpoint-signature', str(unpacked / 'checkpoint.sig'), '--public-key', str(public),
                            '--target', 'desktop', '--repository', release['scope']['repository'],
                            '--previous-state', str(unpacked / 'next-trust-state.json'), '--json']
        subprocess.run(unpacked_channel, capture_output=True, check=True)
        result = subprocess.run(unpack, capture_output=True)
        assert result.returncode != 0 and (unpacked / 'release.json').read_bytes() == payload.read_bytes(), 'Unpack replaced existing output'
        assert prior.read_bytes() == prior_bytes, 'Unpack changed caller replay history'
        print('PASS: CLI channel unpack and downstream verification; replay/tamper/authority/existing-output refusal', flush=True)
        verify(signature, key=wrong_public, expected=False)
        altered = root / "altered.json"
        altered.write_bytes(payload.read_bytes() + b" ")
        verify(signature, manifest=altered, expected=False)
        corrupted = root / "corrupted.iso"
        corrupted.write_bytes(b"abd")
        verify(signature, image=corrupted, expected=False)
        # Exercise the distributed-download workflow through the actual CLI.
        # OpenSSL, not sysroot, supplies its signed release fixture.
        parts = []
        for number, content in enumerate([b"a", b"b", b"c"]):
            path = root / f"installer.part{number}"
            path.write_bytes(content)
            parts.append(path)
        destination = root / "assembled"
        destination.mkdir()
        assemble = [str(binary), "release", "assemble", "--manifest", str(payload),
            "--signature", str(signature), "--public-key", str(public), "--target", "desktop",
            "--output-dir", str(destination), "--json"]
        for invalid in [parts[:-1], list(reversed(parts)), parts + parts]:
            result = subprocess.run(assemble + list(map(str, invalid)), capture_output=True)
            assert result.returncode != 0 and not list(destination.iterdir()), "Bad parts published output"
        result = subprocess.run(assemble + list(map(str, parts)), capture_output=True)
        if result.returncode != 0:
            raise RuntimeError("Installer assembly failed: " + result.stderr.decode())
        assert json.loads(result.stdout)["artifact_verified"]
        assembled = destination / artifact.name
        assert assembled.read_bytes() == artifact.read_bytes()
        verify(signature, image=assembled)
        result = subprocess.run(assemble + list(map(str, parts)), capture_output=True)
        assert result.returncode != 0 and assembled.read_bytes() == b"abc", "Existing output was replaced"
        print("PASS: 16 OpenSSL signatures, wrong-key/tamper/artifact rejection, advisory output", flush=True)
        print("PASS: CLI installer assembly, missing/reordered/extra parts and existing-output refusal", flush=True)
    else:
        print("Generated 16 public signature fixtures for separate-platform CLI verification", flush=True)
finally:
    private.unlink(missing_ok=True)
    wrong_private.unlink(missing_ok=True)
