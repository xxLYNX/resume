#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use domain::{
        Composition, CompositionItem, CompositionSection, ElementKind, SkillKind, SkillNode,
    };
    use store::{import_legacy_content, ImportOptions, Store};

    use crate::{resolve_composition, resolve_composition_with_config, ResolveConfig};

    fn tool_node(id: &str, label: &str) -> SkillNode {
        SkillNode {
            id: id.into(),
            kind: SkillKind::Tool,
            label: label.into(),
            parent_id: None,
            aliases: vec![],
            pdf_groups: vec![],
            framework_refs: serde_json::Value::Null,
        }
    }

    fn test_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn project_payload_technologies_stripped_without_resume_tags() {
        let db = test_root().join("target/resolve-test-projects.db");
        let _ = std::fs::remove_file(&db);
        let store = Store::open(&db).unwrap();
        import_legacy_content(&test_root(), &store, ImportOptions::default()).unwrap();

        let composition = Composition {
            sections: vec![CompositionSection {
                section_key: "projects".into(),
                items: vec![CompositionItem {
                    element_id: "project.ghostrace-cve-research".into(),
                    revision_id: store
                        .list_elements(Some(ElementKind::Project.as_str()))
                        .unwrap()
                        .into_iter()
                        .find(|e| e.id == "project.ghostrace-cve-research")
                        .and_then(|e| e.latest_revision_id)
                        .expect("ghostrace project"),
                    resume_tag_skill_ids: vec![],
                }],
            }],
        };

        let resolved = resolve_composition(&store, &composition).unwrap();
        let projects = resolved.get("projects").unwrap()["projects"]
            .as_array()
            .unwrap();
        let row = &projects[0];
        assert!(row.get("technologies").is_none());
        assert!(row.get("resume_tags").is_none());
    }

    #[test]
    fn project_resume_tags_from_composition_selection() {
        let db = test_root().join("target/resolve-test-resume-tags.db");
        let _ = std::fs::remove_file(&db);
        let store = Store::open(&db).unwrap();
        import_legacy_content(&test_root(), &store, ImportOptions::default()).unwrap();

        store
            .upsert_skill_node(&tool_node("tool.ghidra", "Ghidra"))
            .unwrap();
        store
            .upsert_skill_node(&tool_node("tool.c-lang", "C"))
            .unwrap();
        store
            .set_element_skill_links(
                "project.ghostrace-cve-research",
                &[
                    domain::ElementSkillLink {
                        skill_id: "tool.c-lang".into(),
                        source: "asserted".into(),
                    },
                    domain::ElementSkillLink {
                        skill_id: "tool.ghidra".into(),
                        source: "asserted".into(),
                    },
                ],
            )
            .unwrap();

        let rev_id = store
            .list_elements(Some(ElementKind::Project.as_str()))
            .unwrap()
            .into_iter()
            .find(|e| e.id == "project.ghostrace-cve-research")
            .and_then(|e| e.latest_revision_id)
            .unwrap();

        let composition = Composition {
            sections: vec![CompositionSection {
                section_key: "projects".into(),
                items: vec![CompositionItem {
                    element_id: "project.ghostrace-cve-research".into(),
                    revision_id: rev_id,
                    resume_tag_skill_ids: vec!["tool.ghidra".into(), "tool.c-lang".into()],
                }],
            }],
        };

        let mut config = ResolveConfig::default();
        config.resume_line_tags_max.insert("projects".into(), 5);
        let resolved = resolve_composition_with_config(&store, &composition, &config).unwrap();
        let row = &resolved.get("projects").unwrap()["projects"]
            .as_array()
            .unwrap()[0];
        assert_eq!(row["resume_tags"].as_str(), Some("Ghidra, C"));
        assert!(row.get("technologies").is_none());
    }

    #[test]
    fn rejects_resume_tag_not_on_element() {
        let db = test_root().join("target/resolve-test-invalid-tag.db");
        let _ = std::fs::remove_file(&db);
        let store = Store::open(&db).unwrap();
        import_legacy_content(&test_root(), &store, ImportOptions::default()).unwrap();
        store
            .upsert_skill_node(&tool_node("tool.orphan", "Orphan"))
            .unwrap();

        let rev_id = store
            .list_elements(Some(ElementKind::Project.as_str()))
            .unwrap()
            .into_iter()
            .find(|e| e.id == "project.ghostrace-cve-research")
            .and_then(|e| e.latest_revision_id)
            .unwrap();

        let composition = Composition {
            sections: vec![CompositionSection {
                section_key: "projects".into(),
                items: vec![CompositionItem {
                    element_id: "project.ghostrace-cve-research".into(),
                    revision_id: rev_id,
                    resume_tag_skill_ids: vec!["tool.orphan".into()],
                }],
            }],
        };

        let err = resolve_composition(&store, &composition).unwrap_err();
        assert!(
            format!("{err:#}").contains("not an asserted evidence link"),
            "{err:#}"
        );
    }

    #[test]
    fn rejects_over_max_resume_tags() {
        let db = test_root().join("target/resolve-test-over-max.db");
        let _ = std::fs::remove_file(&db);
        let store = Store::open(&db).unwrap();
        import_legacy_content(&test_root(), &store, ImportOptions::default()).unwrap();

        for (id, label) in [("tool.a", "A"), ("tool.b", "B"), ("tool.c", "C")] {
            store.upsert_skill_node(&tool_node(id, label)).unwrap();
        }
        store
            .set_element_skill_links(
                "project.ghostrace-cve-research",
                &[
                    domain::ElementSkillLink {
                        skill_id: "tool.a".into(),
                        source: "asserted".into(),
                    },
                    domain::ElementSkillLink {
                        skill_id: "tool.b".into(),
                        source: "asserted".into(),
                    },
                    domain::ElementSkillLink {
                        skill_id: "tool.c".into(),
                        source: "asserted".into(),
                    },
                ],
            )
            .unwrap();

        let rev_id = store
            .list_elements(Some(ElementKind::Project.as_str()))
            .unwrap()
            .into_iter()
            .find(|e| e.id == "project.ghostrace-cve-research")
            .and_then(|e| e.latest_revision_id)
            .unwrap();

        let composition = Composition {
            sections: vec![CompositionSection {
                section_key: "projects".into(),
                items: vec![CompositionItem {
                    element_id: "project.ghostrace-cve-research".into(),
                    revision_id: rev_id,
                    resume_tag_skill_ids: vec!["tool.a".into(), "tool.b".into(), "tool.c".into()],
                }],
            }],
        };

        let mut config = ResolveConfig::default();
        config.resume_line_tags_max.insert("projects".into(), 2);
        let err = resolve_composition_with_config(&store, &composition, &config).unwrap_err();
        assert!(format!("{err:#}").contains("at most 2"), "{err:#}");
    }

    #[test]
    fn submitted_manifest_resolved_unchanged_when_skill_links_change() {
        use domain::SubmissionManifest;
        use std::collections::BTreeMap;

        let db = test_root().join("target/resolve-test-manifest-freeze.db");
        let _ = std::fs::remove_file(&db);
        let store = Store::open(&db).unwrap();
        import_legacy_content(&test_root(), &store, ImportOptions::default()).unwrap();

        let composition = Composition {
            sections: vec![CompositionSection {
                section_key: "projects".into(),
                items: vec![CompositionItem {
                    element_id: "project.ghostrace-cve-research".into(),
                    revision_id: store
                        .list_elements(Some(ElementKind::Project.as_str()))
                        .unwrap()
                        .into_iter()
                        .find(|e| e.id == "project.ghostrace-cve-research")
                        .and_then(|e| e.latest_revision_id)
                        .expect("ghostrace"),
                    resume_tag_skill_ids: vec![],
                }],
            }],
        };

        let resolved_map: BTreeMap<String, serde_json::Value> =
            resolve_composition(&store, &composition)
                .unwrap()
                .into_iter()
                .collect();
        let manifest = SubmissionManifest::new(
            "2020-01-01T00:00:00Z".into(),
            "classic".into(),
            None,
            composition.clone(),
            resolved_map.clone(),
        );

        store
            .upsert_skill_node(&tool_node("tool.mcp-freeze", "FreezeTag"))
            .unwrap();
        store
            .set_element_skill_links(
                "project.ghostrace-cve-research",
                &[domain::ElementSkillLink {
                    skill_id: "tool.mcp-freeze".into(),
                    source: "asserted".into(),
                }],
            )
            .unwrap();

        let live: BTreeMap<String, serde_json::Value> = resolve_composition(&store, &composition)
            .unwrap()
            .into_iter()
            .collect();
        let frozen = crate::resolve_from_manifest(&manifest).unwrap();
        assert_ne!(
            live.get("toolbelt"),
            frozen.get("toolbelt"),
            "draft toolbelt should reflect new skill links"
        );
        let frozen_again = crate::resolve_from_manifest(&manifest).unwrap();
        assert_eq!(
            frozen.get("projects"),
            frozen_again.get("projects"),
            "manifest replay must be stable"
        );
    }

    #[test]
    fn project_portfolio_narrative_stripped_on_resolve() {
        use store::CreateElementRequest;

        let db = test_root().join("target/resolve-test-narrative.db");
        let _ = std::fs::remove_file(&db);
        let store = Store::open(&db).unwrap();
        let marker = "PORTFOLIO_NARRATIVE_SHOULD_NOT_RENDER_ON_PDF";
        store
            .create_element(&CreateElementRequest {
                id: "project.narrative-test".into(),
                kind: "project".into(),
                payload: serde_json::json!({
                    "name": "Narrative test",
                    "details": ["Résumé bullet only"],
                    "portfolio_narrative": format!("## Deep dive\n\n{marker}")
                }),
                parent_element_id: None,
            })
            .unwrap();

        let rev_id = store
            .get_element("project.narrative-test")
            .unwrap()
            .unwrap()
            .latest
            .unwrap()
            .id;

        let composition = Composition {
            sections: vec![CompositionSection {
                section_key: "projects".into(),
                items: vec![CompositionItem {
                    element_id: "project.narrative-test".into(),
                    revision_id: rev_id,
                    resume_tag_skill_ids: vec![],
                }],
            }],
        };

        let resolved = resolve_composition(&store, &composition).unwrap();
        let row = &resolved.get("projects").unwrap()["projects"]
            .as_array()
            .unwrap()[0];
        assert!(row.get("portfolio_narrative").is_none());
        assert_eq!(
            row.get("details").unwrap().as_array().unwrap()[0],
            "Résumé bullet only"
        );
    }

    #[test]
    fn project_revision_history_preserves_portfolio_narrative() {
        use store::CreateElementRequest;

        let db = test_root().join("target/resolve-test-narrative-revisions.db");
        let _ = std::fs::remove_file(&db);
        let store = Store::open(&db).unwrap();
        store
            .create_element(&CreateElementRequest {
                id: "project.narrative-rev".into(),
                kind: "project".into(),
                payload: serde_json::json!({
                    "name": "Rev test",
                    "portfolio_narrative": "version one"
                }),
                parent_element_id: None,
            })
            .unwrap();
        store
            .create_revision(
                "project.narrative-rev",
                serde_json::json!({
                    "name": "Rev test",
                    "portfolio_narrative": "version two"
                }),
            )
            .unwrap();

        let detail = store.get_element("project.narrative-rev").unwrap().unwrap();
        assert_eq!(detail.revisions.len(), 2);
        let narratives: Vec<_> = detail
            .revisions
            .iter()
            .map(|r| {
                store
                    .get_revision(&r.id)
                    .unwrap()
                    .unwrap()
                    .payload
                    .get("portfolio_narrative")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert!(narratives.contains(&"version one".to_string()));
        assert!(narratives.contains(&"version two".to_string()));
    }
}
