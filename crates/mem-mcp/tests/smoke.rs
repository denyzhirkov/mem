use mem_mcp::MemServer;

#[test]
fn server_exposes_expected_tools() {
    let server = MemServer::new();
    let tools = server.tool_router_snapshot();

    let names: std::collections::HashSet<&str> = tools.iter().map(|t| t.name.as_ref()).collect();

    for expected in [
        "vault_init",
        "note_new",
        "note_list",
        "note_show",
        "note_update",
        "note_delete",
        "note_related",
        "tags_list",
        "search",
        "sync_status",
        "sync_commit",
        "sync_pull",
        "sync_push",
        "index_rebuild",
    ] {
        assert!(names.contains(expected), "missing tool: {expected}");
    }
}

#[test]
fn all_tools_have_object_output_schema() {
    let server = MemServer::new();
    for t in server.tool_router_snapshot() {
        let schema = t
            .output_schema
            .as_ref()
            .unwrap_or_else(|| panic!("tool {} has no output_schema", t.name));
        let ty = schema.get("type").and_then(|v| v.as_str());
        assert_eq!(
            ty,
            Some("object"),
            "tool {} outputSchema missing type:object; got {}",
            t.name,
            serde_json::to_string(schema).unwrap()
        );
    }
}
