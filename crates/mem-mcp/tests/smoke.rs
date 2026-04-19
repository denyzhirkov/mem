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
