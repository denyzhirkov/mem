import { createSignal, createEffect, onMount, onCleanup, Show, For } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "tiptap-markdown";
import Placeholder from "@tiptap/extension-placeholder";
import "./App.css";

type NoteInfo = {
  id: string;
  title: string;
  slug: string;
  tags: string[];
  updated_at: string;
};

type SearchResult = {
  note_id: string;
  title: string;
  excerpt: string;
  match_kind: string;
  score: number;
};

function App() {
  const [currentNote, setCurrentNote] = createSignal<NoteInfo | null>(null);
  const [noteTitle, setNoteTitle] = createSignal("");
  const [dirty, setDirty] = createSignal(false);
  const [noteTags, setNoteTags] = createSignal<string[]>([]);
  const [relatedNotes, setRelatedNotes] = createSignal<NoteInfo[]>([]);
  const [showPalette, setShowPalette] = createSignal(false);
  const [paletteQuery, setPaletteQuery] = createSignal("");
  const [paletteResults, setPaletteResults] = createSignal<SearchResult[]>([]);
  const [paletteAllNotes, setPaletteAllNotes] = createSignal<NoteInfo[]>([]);
  const [paletteIndex, setPaletteIndex] = createSignal(0);
  const [showDeleteConfirm, setShowDeleteConfirm] = createSignal(false);
  const [theme, setTheme] = createSignal<"light" | "dark">(
    window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
  );

  let editorEl: HTMLDivElement | undefined;
  let paletteInputRef: HTMLInputElement | undefined;
  const [bubbleMenu, setBubbleMenu] = createSignal<{ top: number; left: number } | null>(null);
  let editor: Editor | null = null;
  let charsSinceSave = 0;

  const AUTOSAVE_CHARS = 80;
  const AUTOSAVE_IDLE_MS = 2000;
  const AUTOSAVE_MAX_MS = 30000;
  let idleTimer: any = null;
  let maxTimer: any = null;

  // ===== Theme =====
  createEffect(() => document.documentElement.setAttribute("data-theme", theme()));
  const toggleTheme = () => setTheme(t => t === "light" ? "dark" : "light");

  // ===== Editor =====
  function mountEditor() {
    if (editor || !editorEl) return;
    editor = new Editor({
      element: editorEl,
      extensions: [
        StarterKit,
        Markdown,
        Placeholder.configure({ placeholder: "Just start writing..." }),
      ],
      content: "",
      autofocus: "end",
      editorProps: { attributes: { class: "tiptap" } },
      onUpdate: ({ transaction }) => {
        setDirty(true);
        transaction.steps.forEach((step: any) => {
          if (step.slice?.content?.size) {
            charsSinceSave += step.slice.content.size;
          }
        });
        scheduleAutosave();
      },
      onSelectionUpdate: ({ editor: e }) => {
        if (e.state.selection.empty) {
          setBubbleMenu(null);
          return;
        }
        const { from, to } = e.state.selection;
        const start = e.view.coordsAtPos(from);
        const end = e.view.coordsAtPos(to);
        const editorRect = editorEl!.getBoundingClientRect();
        setBubbleMenu({
          top: start.top - editorRect.top - 40,
          left: (start.left + end.left) / 2 - editorRect.left,
        });
      },
    });
  }

  function getEditorMarkdown(): string {
    if (!editor) return "";
    return (editor.storage as any).markdown.getMarkdown();
  }

  // ===== Autosave =====
  function scheduleAutosave() {
    // Idle debounce: save after 2s of no typing
    clearTimeout(idleTimer);
    idleTimer = setTimeout(() => doAutosave(), AUTOSAVE_IDLE_MS);

    // Char threshold: save every N chars
    if (charsSinceSave >= AUTOSAVE_CHARS) {
      doAutosave();
      return;
    }

    // Max interval: save at least every 30s if dirty
    if (!maxTimer) {
      maxTimer = setTimeout(() => {
        maxTimer = null;
        if (dirty()) doAutosave();
      }, AUTOSAVE_MAX_MS);
    }
  }

  async function doAutosave() {
    clearTimeout(idleTimer);
    charsSinceSave = 0;
    if (dirty()) await saveNote();
  }

  // ===== Data =====
  async function loadPaletteData() {
    try {
      if (paletteQuery().length > 0) {
        const results = await invoke<SearchResult[]>("search_notes", { query: paletteQuery() });
        setPaletteResults(results);
        setPaletteAllNotes([]);
      } else {
        const all = await invoke<NoteInfo[]>("list_notes");
        setPaletteAllNotes(all);
        setPaletteResults([]);
      }
      setPaletteIndex(0);
    } catch (e) {
      console.error("loadPaletteData:", e);
    }
  }

  // Total items count in palette
  const paletteItemCount = () => {
    if (paletteQuery().length > 0) return paletteResults().length;
    return paletteAllNotes().length;
  };

  // Get note info for a palette index
  function paletteNoteAtIndex(idx: number): NoteInfo | null {
    if (paletteQuery().length > 0) {
      const r = paletteResults()[idx];
      return r ? { id: r.note_id, title: r.title, slug: "", tags: [], updated_at: "" } : null;
    }
    return paletteAllNotes()[idx] || null;
  }

  async function loadNoteDetails(noteId: string) {
    try {
      const [tags, related] = await Promise.all([
        invoke<string[]>("get_note_tags", { id: noteId }),
        invoke<NoteInfo[]>("get_related_notes", { id: noteId }),
      ]);
      setNoteTags(tags);
      setRelatedNotes(related);
    } catch (e) {
      console.error(e);
    }
  }

  // ===== Note operations =====
  function newBlankNote() {
    setCurrentNote(null);
    setNoteTitle("");
    setDirty(false);
    setNoteTags([]);
    setRelatedNotes([]);
    setShowDeleteConfirm(false);
    charsSinceSave = 0;
    if (editor) {
      editor.commands.clearContent();
      editor.commands.focus("start");
    }
  }

  async function openNote(note: NoteInfo) {
    if (dirty()) await saveNote();
    setCurrentNote(note);
    setNoteTitle(note.title);
    setDirty(false);
    setShowDeleteConfirm(false);
    setShowPalette(false);
    charsSinceSave = 0;

    try {
      const content = await invoke<string>("get_note", { id: note.id });
      if (editor) {
        editor.commands.setContent(content);
        editor.commands.focus("end");
      }
    } catch (e) {
      console.error("openNote:", e);
      if (editor) editor.commands.setContent("");
    }
    loadNoteDetails(note.id);
  }

  function generateAutoTitle(): string {
    const md = getEditorMarkdown();
    const firstLine = md.split("\n").find(l => l.replace(/^#+\s*/, "").trim().length > 0);
    if (firstLine) {
      const clean = firstLine.replace(/^#+\s*/, "").trim();
      if (clean.length > 0) return clean.slice(0, 60);
    }
    const now = new Date();
    const pad = (n: number) => n.toString().padStart(2, "0");
    return `note_${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}_${pad(now.getHours())}${pad(now.getMinutes())}`;
  }

  async function saveNote() {
    if (!editor) return;
    const content = getEditorMarkdown();
    if (!content.trim() && !currentNote()) return;

    const title = noteTitle().trim() || generateAutoTitle();
    setNoteTitle(title);

    try {
      if (currentNote()) {
        const updated = await invoke<NoteInfo>("update_note", { id: currentNote()!.id, title, content });
        setCurrentNote(updated);
        setDirty(false);
        loadNoteDetails(updated.id);
      } else {
        const created = await invoke<NoteInfo>("create_note", { title });
        const updated = await invoke<NoteInfo>("update_note", { id: created.id, title, content });
        setCurrentNote(updated);
        setDirty(false);
        loadNoteDetails(updated.id);
      }
    } catch (e) {
      console.error("saveNote:", e);
    }
  }

  async function deleteNote() {
    const note = currentNote();
    if (!note) return;
    try {
      await invoke("remove_note", { id: note.id });
      newBlankNote();
    } catch (e) {
      console.error("deleteNote:", e);
    }
  }

  // ===== Palette =====
  function openPalette() {
    setShowPalette(true);
    setPaletteQuery("");
    setPaletteIndex(0);
    loadPaletteData();
    setTimeout(() => paletteInputRef?.focus(), 30);
  }

  function closePalette() {
    setShowPalette(false);
    editor?.commands.focus();
  }

  function paletteKeyDown(e: KeyboardEvent) {
    const count = paletteItemCount();
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setPaletteIndex(i => Math.min(i + 1, count - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setPaletteIndex(i => Math.max(i - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const note = paletteNoteAtIndex(paletteIndex());
      if (note) {
        openNote(note);
      } else {
        const q = paletteQuery().trim();
        if (q) { newBlankNote(); setNoteTitle(q); closePalette(); }
      }
    } else if (e.key === "Escape") {
      closePalette();
    }
  }

  function handleTagClick(tag: string) {
    openPalette();
    setTimeout(() => { setPaletteQuery(tag); loadPaletteData(); }, 50);
  }

  // ===== Lifecycle =====
  onMount(() => {
    mountEditor();
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") { e.preventDefault(); saveNote(); }
      else if ((e.metaKey || e.ctrlKey) && e.key === "p") { e.preventDefault(); showPalette() ? closePalette() : openPalette(); }
      else if ((e.metaKey || e.ctrlKey) && e.key === "n") { e.preventDefault(); if (dirty()) saveNote().then(newBlankNote); else newBlankNote(); }
    };
    window.addEventListener("keydown", handleKeyDown);
    onCleanup(() => {
      window.removeEventListener("keydown", handleKeyDown);
      clearTimeout(idleTimer);
      clearTimeout(maxTimer);
      editor?.destroy();
    });
  });

  let palSearchTimeout: any;
  function handlePaletteInput(val: string) {
    setPaletteQuery(val);
    clearTimeout(palSearchTimeout);
    palSearchTimeout = setTimeout(() => loadPaletteData(), 200);
  }

  // ===== Helpers =====
  const formatDate = () => {
    const note = currentNote();
    if (!note?.updated_at) return "";
    try {
      return new Date(note.updated_at).toLocaleDateString("en-US", {
        month: "short", day: "numeric", year: "numeric", hour: "2-digit", minute: "2-digit",
      });
    } catch { return ""; }
  };

  /** Render FTS excerpt with <m>...</m> highlights */
  function ExcerptHtml(props: { text: string; kind: string }) {
    // FTS returns <m>match</m> markers
    const parts = props.text.split(/(<m>.*?<\/m>)/g);
    return (
      <span class="excerpt-text">
        <For each={parts}>
          {(part) => {
            const match = part.match(/^<m>(.*)<\/m>$/);
            if (match) {
              if (props.kind === "tag") {
                return <span class="excerpt-tag-hl">{match[1]}</span>;
              }
              return <span class="excerpt-hl">{match[1]}</span>;
            }
            return <span>{part}</span>;
          }}
        </For>
      </span>
    );
  }

  return (
    <div class="app">
      {/* Top bar */}
      <div class="topbar">
        <div class="topbar-left">
          <span class="brand">mem</span>
        </div>
        <div class="topbar-right">
          <button class="topbar-btn theme-toggle" onClick={toggleTheme}>
            {theme() === "light" ? "\u263E" : "\u2600"}
          </button>
          <Show when={dirty()}>
            <button class="topbar-btn" onClick={saveNote}>
              Save <div class="save-dot" />
            </button>
          </Show>
          <Show when={currentNote()}>
            <button class="topbar-btn" onClick={() => setShowDeleteConfirm(true)} style={{ color: "var(--danger)" }}>
              {"\u2715"}
            </button>
          </Show>
          <button class="topbar-btn" onClick={openPalette}>
            Notes <kbd>{"\u2318"}P</kbd>
          </button>
          <button class="topbar-btn" onClick={() => { if (dirty()) saveNote().then(newBlankNote); else newBlankNote(); }}>
            New <kbd>{"\u2318"}N</kbd>
          </button>
        </div>
      </div>

      {/* Canvas */}
      <div class="canvas">
        <div class="page">
          <input
            class="title-input"
            type="text"
            placeholder="Untitled"
            value={noteTitle()}
            onInput={(e) => { setNoteTitle(e.currentTarget.value); setDirty(true); }}
          />
          <Show when={currentNote()}>
            <div class="title-date">{formatDate()}</div>
          </Show>
          <div class="editor-wrap">
            <Show when={bubbleMenu()}>
              <div class="bubble-menu" style={{ top: `${bubbleMenu()!.top}px`, left: `${bubbleMenu()!.left}px` }}>
                <button class="bubble-btn" onMouseDown={(e) => { e.preventDefault(); editor?.chain().focus().toggleBold().run(); }} classList={{ active: editor?.isActive("bold") }}>B</button>
                <button class="bubble-btn" onMouseDown={(e) => { e.preventDefault(); editor?.chain().focus().toggleItalic().run(); }} classList={{ active: editor?.isActive("italic") }}><em>I</em></button>
                <button class="bubble-btn" onMouseDown={(e) => { e.preventDefault(); editor?.chain().focus().toggleStrike().run(); }} classList={{ active: editor?.isActive("strike") }}><s>S</s></button>
                <button class="bubble-btn" onMouseDown={(e) => { e.preventDefault(); editor?.chain().focus().toggleCode().run(); }} classList={{ active: editor?.isActive("code") }}>&lt;/&gt;</button>
              </div>
            </Show>
            <div class="editor-mount" ref={editorEl!} />
          </div>
        </div>
      </div>

      {/* Tags + Related — sticky bottom */}
      <Show when={currentNote() && (noteTags().length > 0 || relatedNotes().length > 0)}>
        <div class="note-footer">
          <Show when={noteTags().length > 0}>
            <div class="footer-section">
              <div class="footer-label">Tags</div>
              <div class="tag-list">
                <For each={noteTags()}>
                  {(tag) => <button class="tag" onClick={() => handleTagClick(tag)}>#{tag}</button>}
                </For>
              </div>
            </div>
          </Show>
          <Show when={relatedNotes().length > 0}>
            <div class="footer-section">
              <div class="footer-label">Related</div>
              <For each={relatedNotes()}>
                {(rn) => (
                  <div class="related-item" onClick={() => openNote(rn)}>
                    <span class="related-title">{rn.title}</span>
                    <div class="related-tags">
                      <For each={rn.tags.slice(0, 2)}>
                        {(t) => <span class="tag-mini">#{t}</span>}
                      </For>
                    </div>
                    <span class="related-arrow">{"\u2192"}</span>
                  </div>
                )}
              </For>
            </div>
          </Show>
        </div>
      </Show>

      {/* Status bar */}
      <div class="statusbar">
        <div class="statusbar-hints">
          <span># heading</span>
          <span>**bold**</span>
          <span>*italic*</span>
          <span>- list</span>
          <span>&gt; quote</span>
          <span>#tag</span>
        </div>
        <span>{currentNote() ? "Saved" : "New note"}</span>
      </div>

      {/* Delete confirm */}
      <Show when={showDeleteConfirm()}>
        <div class="delete-bar">
          <span>Delete "{currentNote()?.title || "this note"}"?</span>
          <button class="btn-sm btn-danger" onClick={() => { deleteNote(); setShowDeleteConfirm(false); }}>Delete</button>
          <button class="btn-sm btn-cancel" onClick={() => setShowDeleteConfirm(false)}>Cancel</button>
        </div>
      </Show>

      {/* Command Palette */}
      <Show when={showPalette()}>
        <div class="palette-overlay" onClick={(e) => { if (e.target === e.currentTarget) closePalette(); }}>
          <div class="palette">
            <input
              ref={paletteInputRef}
              class="palette-input"
              type="text"
              placeholder="Search notes..."
              value={paletteQuery()}
              onInput={(e) => handlePaletteInput(e.currentTarget.value)}
              onKeyDown={paletteKeyDown}
            />
            <div class="palette-list">
              <div class="palette-action" onClick={() => { newBlankNote(); closePalette(); }}>
                + New blank note
              </div>

              {/* Search results mode */}
              <Show when={paletteQuery().length > 0}>
                <For each={paletteResults()}>
                  {(result, i) => (
                    <div
                      class={`palette-item ${paletteIndex() === i() ? "active" : ""}`}
                      onClick={() => { const n = paletteNoteAtIndex(i()); if (n) openNote(n); }}
                      onMouseEnter={() => setPaletteIndex(i())}
                    >
                      <div class="palette-item-body">
                        <div class="palette-item-title">{result.title}</div>
                        <Show when={result.excerpt}>
                          <div class={`palette-item-excerpt ${result.match_kind === "tag" ? "excerpt-tag" : ""}`}>
                            <ExcerptHtml text={result.excerpt} kind={result.match_kind} />
                          </div>
                        </Show>
                      </div>
                      <span class={`palette-match-badge ${result.match_kind}`}>{result.match_kind}</span>
                    </div>
                  )}
                </For>
                <Show when={paletteResults().length === 0}>
                  <div class="palette-empty">No notes found. Press Enter to create one.</div>
                </Show>
              </Show>

              {/* Browse mode — all notes */}
              <Show when={paletteQuery().length === 0}>
                <For each={paletteAllNotes()}>
                  {(note, i) => (
                    <div
                      class={`palette-item ${paletteIndex() === i() ? "active" : ""}`}
                      onClick={() => openNote(note)}
                      onMouseEnter={() => setPaletteIndex(i())}
                    >
                      <div class="palette-item-body">
                        <span class="palette-item-title">{note.title}</span>
                      </div>
                      <div class="palette-item-tags">
                        <For each={note.tags.slice(0, 3)}>
                          {(t) => <span class="tag-mini">#{t}</span>}
                        </For>
                      </div>
                    </div>
                  )}
                </For>
              </Show>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}

export default App;
