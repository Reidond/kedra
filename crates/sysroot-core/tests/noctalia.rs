use sysroot_core::noctalia::*;

#[test]
fn pending_activation_reserves_mutations_and_releases_without_changing_dispositions() {
    let mut review = state(Settings {
        theme_mode: Theme::Light,
        ..base()
    });
    review.stage(Key::ThemeMode).unwrap();
    let before = review.to_bytes().unwrap();
    let token = "a".repeat(32);
    review.reserve_activation(&token).unwrap();
    let mut reopened = State::from_bytes(&review.to_bytes().unwrap(), &"0".repeat(32)).unwrap();
    assert_eq!(reopened.pending_activation(), Some(token.as_str()));
    assert_eq!(reopened.capture(base()), Err(Error::ActivationPending));
    assert_eq!(
        reopened.unstage(Key::ThemeMode),
        Err(Error::ActivationPending)
    );
    assert_eq!(
        reopened.own(Key::InputBorders),
        Err(Error::ActivationPending)
    );
    assert_eq!(
        reopened.release_activation(&"b".repeat(32)),
        Err(Error::StaleTransition)
    );
    reopened.release_activation(&token).unwrap();
    assert_eq!(reopened.to_bytes().unwrap(), before);
    assert!(
        !String::from_utf8(before)
            .unwrap()
            .contains("pending_activation")
    );
}

#[test]
fn discard_targets_unstaged_values_and_preserves_other_dispositions() {
    let mut review = state(Settings {
        theme_mode: Theme::Light,
        button_borders: false,
        input_borders: false,
    });
    review.stage(Key::ThemeMode).unwrap();
    review.ignore_exact(Key::ButtonBorders).unwrap();
    review
        .capture(Settings {
            theme_mode: Theme::Auto,
            button_borders: false,
            input_borders: false,
        })
        .unwrap();
    let selected = review.selection().unwrap();
    let before = review.to_bytes().unwrap();
    let desired = review.discarded_settings(Key::ThemeMode).unwrap();
    assert_eq!(
        desired,
        Settings {
            theme_mode: Theme::Light,
            button_borders: false,
            input_borders: false
        }
    );
    assert_eq!(review.to_bytes().unwrap(), before);
    review.capture(desired).unwrap();
    assert_eq!(review.selection().unwrap(), selected);
    assert!(row(&review, Key::ButtonBorders).local_only);
    review.own(Key::InputBorders).unwrap();
    assert_eq!(
        review.discarded_settings(Key::InputBorders),
        Err(Error::PolicyOverlap)
    );
}

fn base() -> Settings {
    Settings {
        theme_mode: Theme::Dark,
        button_borders: true,
        input_borders: true,
    }
}
fn baseline(settings: Settings, revision: char) -> Baseline {
    Baseline {
        target: "desktop".into(),
        source_path: "home/.config/noctalia/config.toml".into(),
        source_revision: revision.to_string().repeat(40),
        settings,
    }
}
fn state(live: Settings) -> State {
    State::new("0".repeat(32), baseline(base(), 'a'), live).unwrap()
}
fn row(state: &State, key: Key) -> Row {
    state
        .rows()
        .unwrap()
        .into_iter()
        .find(|r| r.key == key)
        .unwrap()
}

#[test]
fn projection_discards_unknown_private_values_before_serialization() {
    let export = "[theme]\nmode='light'\n[shell]\nbutton_borders=false\ninput_borders=true\n[credentials]\naccess_token='SYNTHETIC_SECRET_MUST_NOT_CAPTURE'\n";
    let projection = project(APP_VERSION, export).unwrap();
    assert_eq!(projection.theme_mode, Theme::Light);
    let serialized = serde_json::to_string(&projection).unwrap();
    assert!(!serialized.contains("SECRET"));
    assert!(!serialized.contains("credentials"));
    assert!(project("4.9.0", export).is_err());
    assert!(project("5.0.2", export).is_err());
    let malformed = "[theme]\nmode='SYNTHETIC_SECRET_MUST_NOT_LOG'\n";
    let error = project(APP_VERSION, malformed).unwrap_err().to_string();
    assert!(!error.contains("SECRET"));
    assert!(project(APP_VERSION, "[theme]\nmode='dark'\n").is_err());
}

#[test]
fn selected_snapshot_stays_fixed_while_other_dispositions_coexist() {
    let live = Settings {
        theme_mode: Theme::Light,
        button_borders: false,
        input_borders: false,
    };
    let mut s = state(live);
    s.stage(Key::ThemeMode).unwrap();
    s.ignore_exact(Key::ButtonBorders).unwrap();
    let selected = s.selection().unwrap();
    assert_eq!(selected.len(), 1);
    assert_eq!(selected[0].key, Key::ThemeMode);
    s.capture(Settings {
        theme_mode: Theme::Auto,
        ..live
    })
    .unwrap();
    assert_eq!(s.selection().unwrap(), selected);
    assert!(row(&s, Key::ThemeMode).visible_change);
    assert!(row(&s, Key::ButtonBorders).local_only);
    assert!(!row(&s, Key::ButtonBorders).visible_change);
    assert!(row(&s, Key::InputBorders).visible_change);
    assert_eq!(
        State::from_bytes(&s.to_bytes().unwrap(), &"0".repeat(32)).unwrap(),
        s
    );
}

#[test]
fn exact_ignore_expires_on_new_value_but_app_ownership_persists() {
    let live = Settings {
        theme_mode: Theme::Light,
        ..base()
    };
    let mut s = state(live);
    s.ignore_exact(Key::ThemeMode).unwrap();
    let newer = Settings {
        theme_mode: Theme::Auto,
        ..base()
    };
    assert_eq!(s.capture(newer).unwrap(), [Key::ThemeMode]);
    assert!(row(&s, Key::ThemeMode).visible_change);
    s.own(Key::ThemeMode).unwrap();
    s.capture(live).unwrap();
    assert!(!row(&s, Key::ThemeMode).visible_change);
    assert!(row(&s, Key::ThemeMode).app_owned);
    assert_eq!(s.stage(Key::ThemeMode), Err(Error::PolicyOverlap));
    s.clear_local_policy(Key::ThemeMode).unwrap();
    s.stage(Key::ThemeMode).unwrap();
}

#[test]
fn overlapping_decisions_fail_in_both_orders_without_state_changes() {
    for stage_first in [true, false] {
        let mut s = state(Settings {
            theme_mode: Theme::Light,
            ..base()
        });
        if stage_first {
            s.stage(Key::ThemeMode).unwrap();
        } else {
            s.ignore_exact(Key::ThemeMode).unwrap();
        }
        let before = s.to_bytes().unwrap();
        let result = if stage_first {
            s.ignore_exact(Key::ThemeMode)
        } else {
            s.stage(Key::ThemeMode)
        };
        assert_eq!(result, Err(Error::PolicyOverlap));
        assert_eq!(s.to_bytes().unwrap(), before);
    }
}

#[test]
fn published_selection_is_not_reexported_and_newer_gui_write_survives_deployment() {
    let mut s = state(Settings {
        theme_mode: Theme::Light,
        button_borders: false,
        input_borders: false,
    });
    s.stage(Key::ThemeMode).unwrap();
    s.ignore_exact(Key::ButtonBorders).unwrap();
    let published = Settings {
        theme_mode: Theme::Light,
        ..base()
    };
    s.record_source_commit(&"b".repeat(40), published).unwrap();
    assert!(s.selection().unwrap().is_empty());
    assert!(!row(&s, Key::ThemeMode).visible_change);
    s.capture(Settings {
        theme_mode: Theme::Auto,
        button_borders: false,
        input_borders: false,
    })
    .unwrap();
    let before = s.to_bytes().unwrap();
    let transition = s.prepare(baseline(published, 'b')).unwrap();
    assert_eq!(s.to_bytes().unwrap(), before); // Preparation never advances B.
    assert_eq!(transition.desired_live().theme_mode, Theme::Auto);
    let observed = transition.desired_live();
    s.accept(transition, observed).unwrap();
    assert_eq!(row(&s, Key::ThemeMode).committed_pending, None);
    assert!(row(&s, Key::ThemeMode).visible_change);
    assert!(row(&s, Key::ButtonBorders).local_only);
}

#[test]
fn intermediate_deployment_consumes_only_the_matching_publication_prefix() {
    let mut s = state(Settings {
        theme_mode: Theme::Light,
        ..base()
    });
    s.stage(Key::ThemeMode).unwrap();
    let first = Settings {
        theme_mode: Theme::Light,
        ..base()
    };
    s.record_source_commit(&"b".repeat(40), first).unwrap();
    let second = Settings {
        theme_mode: Theme::Auto,
        ..base()
    };
    s.capture(second).unwrap();
    s.stage(Key::ThemeMode).unwrap();
    s.record_source_commit(&"c".repeat(40), second).unwrap();
    s.capture(base()).unwrap(); // Newer GUI reversal must remain visible.
    let transition = s.prepare(baseline(first, 'b')).unwrap();
    assert_eq!(transition.desired_live(), base());
    s.accept(transition, base()).unwrap();
    assert_eq!(
        row(&s, Key::ThemeMode).committed_pending,
        Some(Value::Theme(Theme::Auto))
    );
    let transition = s.prepare(baseline(second, 'c')).unwrap();
    s.accept(transition, base()).unwrap();
    assert_eq!(row(&s, Key::ThemeMode).committed_pending, None);
    assert!(row(&s, Key::ThemeMode).visible_change);
}

#[test]
fn unrelated_baseline_changes_rebase_staging_and_owned_fields_keep_live_values() {
    let mut s = state(Settings {
        theme_mode: Theme::Light,
        ..base()
    });
    s.stage(Key::ThemeMode).unwrap();
    s.own(Key::InputBorders).unwrap();
    let next = Settings {
        button_borders: false,
        input_borders: false,
        ..base()
    };
    let transition = s.prepare(baseline(next, 'b')).unwrap();
    let desired = transition.desired_live();
    assert_eq!(
        desired,
        Settings {
            theme_mode: Theme::Light,
            button_borders: false,
            input_borders: true
        }
    );
    s.accept(transition, desired).unwrap();
    assert_eq!(s.selection().unwrap()[0].after, Value::Theme(Theme::Light));
    assert!(row(&s, Key::InputBorders).app_owned);
}

#[test]
fn conflicting_next_baseline_preserves_every_disposition_and_accepted_base() {
    for decision in [0, 1, 2] {
        let mut s = state(Settings {
            theme_mode: Theme::Light,
            ..base()
        });
        match decision {
            0 => s.stage(Key::ThemeMode).unwrap(),
            1 => s.ignore_exact(Key::ThemeMode).unwrap(),
            _ => {
                s.stage(Key::ThemeMode).unwrap();
                s.record_source_commit(
                    &"b".repeat(40),
                    Settings {
                        theme_mode: Theme::Light,
                        ..base()
                    },
                )
                .unwrap();
            }
        }
        let before = s.to_bytes().unwrap();
        let result = s.prepare(baseline(
            Settings {
                theme_mode: Theme::Auto,
                ..base()
            },
            'c',
        ));
        assert!(matches!(result, Err(Error::Conflict(Key::ThemeMode))));
        assert_eq!(s.to_bytes().unwrap(), before);
    }
}

#[test]
fn source_receipts_and_activation_rechecks_cannot_consume_stale_selection() {
    let mut s = state(Settings {
        theme_mode: Theme::Light,
        ..base()
    });
    s.stage(Key::ThemeMode).unwrap();
    let before = s.to_bytes().unwrap();
    assert_eq!(
        s.record_source_commit(&"b".repeat(40), base()),
        Err(Error::SourceChanged)
    );
    assert_eq!(s.to_bytes().unwrap(), before);
    let transition = s.prepare(baseline(base(), 'b')).unwrap();
    s.capture(Settings {
        theme_mode: Theme::Auto,
        ..base()
    })
    .unwrap();
    assert_eq!(s.accept(transition, base()), Err(Error::StaleTransition));
    let transition = s.prepare(baseline(base(), 'b')).unwrap();
    assert_eq!(s.accept(transition, base()), Err(Error::StaleTransition));
}

#[test]
fn unknown_corrupt_or_cross_machine_state_never_becomes_an_empty_baseline() {
    let s = state(base());
    let bytes = s.to_bytes().unwrap();
    assert_eq!(
        State::from_bytes(&bytes, &"1".repeat(32)),
        Err(Error::ScopeChanged)
    );
    assert_eq!(
        State::from_bytes(b"{}", &"0".repeat(32)),
        Err(Error::CorruptState)
    );
    let text = String::from_utf8(bytes.clone()).unwrap();
    for bad in [
        text.replace("\"schema_version\":1", "\"schema_version\":2"),
        text.replacen('{', "{\"schema_version\":1,", 1),
        text.replace(
            "\"selected\":{}",
            "\"selected\":{\"unknown-secret-field\":true}",
        ),
        text.trim_end().to_owned(),
    ] {
        assert!(State::from_bytes(bad.as_bytes(), &"0".repeat(32)).is_err());
    }
    let mut other = baseline(base(), 'b');
    other.target = "xps".into();
    assert!(matches!(s.prepare(other), Err(Error::ScopeChanged)));
}
