#!/usr/bin/env bash
# Native Fedora evidence only. No solver, key import, source edits or publication.
set -euo pipefail
export LC_ALL=C.UTF-8
evidence=/evidence
cache=/var/cache/kedra-refresh
rpm_format='%{NAME}\t%{EPOCHNUM}\t%{VERSION}\t%{RELEASE}\t%{ARCH}\t%{SHA256HEADER}\t%{PAYLOADSHA256}\t%{VENDOR}\t%{SOURCERPM}\n'
test -f /run/.containerenv
test -f "$evidence/request.json"
jq -e '.schema_version == 1 and .scope == "desktop-resolution-research" and .freshness_written == false' "$evidence/request.json" >/dev/null

# Options only redirect/retain the cache and make the intended TLS/package checks
# explicit. Package policy and upgrade/install/remove ordering come from assemble.sh.
options=(--repo=fedora --repo=updates '--setopt=*.skip_if_unavailable=False'
    '--setopt=*.gpgcheck=True' '--setopt=*.sslverify=True'
    --setopt=keepcache=True "--setopt=system_cachedir=$cache")

native() {
    local label=$1 code started finished
    shift
    mkdir "$evidence/commands/$label"
    jq -cn --args '$ARGS.positional' -- "$@" > "$evidence/commands/$label/argv.json"
    started=$(date +%s%N)
    set +e
    "$@" > "$evidence/commands/$label/stdout" 2> "$evidence/commands/$label/stderr"
    code=$?
    set -e
    finished=$(date +%s%N)
    jq -cn --argjson code "$code" --arg started "$started" --arg finished "$finished" \
        '{exit_code:$code,started_ns:$started,finished_ns:$finished}' > "$evidence/commands/$label/result.json"
    if test "$code" -ne 0; then
        cat "$evidence/commands/$label/stderr" >&2
        printf 'Native %s exited %s\n' "$label" "$code" >&2
    fi
    return "$code"
}

inventory() {
    local label=$1
    native "$label" rpm -qa --queryformat "$rpm_format"
    LC_ALL=C sort "$evidence/commands/$label/stdout" > "$evidence/$label.tsv"
}

snapshot_cache() {
    local label=$1 path relative hash destination
    mkdir "$evidence/metadata/$label"
    if ! test -d "$cache"; then return; fi
    # Runtime cache artifacts, never repository source or a host configuration tree.
    while IFS= read -r -d '' path; do
        relative=${path#"$cache/"}
        if [[ "$relative" == *.rpm ]]; then
            hash=$(sha256sum "$path" | cut -d ' ' -f 1)
            destination="$evidence/rpms/$hash.rpm"
            if ! test -e "$destination"; then
                cp -- "$path" "$destination"
                # Requiring all prevents a digest-only unsigned archive from being
                # mistaken for an authenticated package. DNF also checks its RPMs.
                native "signature-$hash" rpmkeys --define '_pkgverify_level all' --checksig --verbose "$destination"
                native "header-$hash" rpm -qp --queryformat "$rpm_format" "$destination"
            fi
            jq -cn --arg step "$label" --arg path "$relative" --arg sha256 "$hash" \
                '{step:$step,cache_path:$path,sha256:$sha256}' >> "$evidence/archive-origins.jsonl"
        else
            destination="$evidence/metadata/$label/$relative"
            mkdir -p -- "$(dirname "$destination")"
            cp -- "$path" "$destination"
        fi
    done < <(find "$cache" -type f -print0)
}

record_repositories() {
    local label=$1
    native "$label-main-config" /usr/bin/dnf "${options[@]}" --dump-main-config
    native "$label-repo-config" /usr/bin/dnf "${options[@]}" --dump-repo-config=fedora,updates
    native "$label-variables" /usr/bin/dnf "${options[@]}" --dump-variables
    native "$label-repos" /usr/bin/dnf "${options[@]}" --cacheonly repo info --json --enabled
    native "$label-keys" rpmkeys --list
    native "$label-public-keys" rpmkeys --export
}

observe_dnf() {
    local operation='' argument code
    for argument in "$@"; do
        case "$argument" in
            -*) ;;
            upgrade|install|remove|clean|check) operation=$argument; break ;;
            *) printf 'Unexpected research DNF operation\n' >&2; return 1 ;;
        esac
    done
    test -n "$operation"
    # Each operation must really run once. The retained native receipt and mkdir
    # refusal also prevent an accidental duplicate or cached success claim.
    if test "$operation" = clean; then
        record_repositories final
        native final-reasons /usr/bin/dnf "${options[@]}" --cacheonly repoquery --installed \
            --queryformat '%{full_nevra}\t%{from_repo}\t%{reason}\n'
        snapshot_cache before-clean
    fi
    inventory "$operation-before"
    code=0
    native "$operation" /usr/bin/dnf "${options[@]}" "$@" || code=$?
    inventory "$operation-after"
    if test "$code" -ne 0; then
        # Preserve the real DNF error before attempting optional cache inspection.
        jq -cn --arg step "$operation" --argjson code "$code" \
            '{step:$step,native_exit_code:$code}' > "$evidence/native-failure.json"
        return "$code"
    fi
    case "$operation" in
        upgrade|install|remove)
            record_repositories "$operation"
            snapshot_cache "$operation"
            ;;
    esac
}

finish() {
    local code=$? state=failed
    trap - EXIT
    if test "$code" -eq 0; then state=succeeded; fi
    jq --arg state "$state" --arg finished "$(date +%s%N)" --argjson code "$code" \
        '.state=$state | .finished_ns=$finished | .process_exit_code=$code' \
        "$evidence/outcome.json" > "$evidence/outcome.next"
    mv "$evidence/outcome.next" "$evidence/outcome.json"
    exit "$code"
}

case "${1:-}" in
    inspect)
        # Read only the explicit generated payload inventory. This is artifact
        # validation, not a filesystem normalization/equivalence claim.
        while IFS= read -r -d '' path; do
            test -f "/$path" && test ! -L "/$path"
            hash=$(sha256sum "/$path" | cut -d ' ' -f 1)
            mode=$(stat -c '%a' "/$path")
            uid=$(stat -c '%u' "/$path")
            gid=$(stat -c '%g' "/$path")
            jq -cn --arg path "$path" --arg hash "$hash" --arg mode "$mode" --argjson uid "$uid" --argjson gid "$gid" \
                '{path:$path,kind:"file",sha256:$hash,mode:$mode,uid:$uid,gid:$gid}'
        done < <(jq -jr 'keys[] | ., "\u0000"' "$evidence/expected-files.json")
        for path in etc/systemd/system/default.target \
            etc/systemd/system/bootc-fetch-apply-updates.timer \
            etc/systemd/system/bootc-fetch-apply-updates.service; do
            test -L "/$path"
            jq -cn --arg path "$path" --arg target "$(readlink "/$path")" \
                '{path:$path,kind:"symlink",target:$target}'
        done
        ;;
    dnf)
        shift
        observe_dnf "$@"
        ;;
    run)
        mkdir "$evidence/commands" "$evidence/metadata" "$evidence/rpms"
        test ! -e "$cache"
        jq -cn --arg execution "$(cat /proc/sys/kernel/random/uuid)" --arg started "$(date +%s%N)" \
            '{schema_version:1,execution_id:$execution,started_ns:$started,state:"running",whole_image_equivalence:"unproven",freshness_written:false}' > "$evidence/outcome.json"
        trap finish EXIT
        cp /usr/share/sysroot/source.json "$evidence/source.before.json"
        jq -cn --args '$ARGS.positional' -- "${options[@]}" > "$evidence/observer-dnf-options.json"
        native dnf-version /usr/bin/dnf --version
        native rpm-version rpm --version
        native rpm-querytags rpm --querytags
        native rpm-config rpm --showrc
        native initial-keys rpmkeys --list
        native initial-public-keys rpmkeys --export
        inventory initial
        PATH="/opt/kedra-refresh/bin:$PATH" /bin/bash /tmp/kedra-assemble.sh
        inventory final
        cp /usr/share/sysroot/source.json "$evidence/source.after.json"
        cp /usr/share/sysroot/packages.txt "$evidence/packages.txt"
        # Native installed-file differences are retained for review. Intentional
        # desktop/config overlays mean exit 1 is not itself an assembly failure,
        # and neither exit 0 nor 1 establishes whole-image equivalence.
        verification=0
        native rpm-verify-all rpm --verify --all || verification=$?
        test "$verification" -le 1
        ;;
    *) printf 'Use the Actions research run/dnf/inspect entrypoint\n' >&2; exit 1 ;;
esac
