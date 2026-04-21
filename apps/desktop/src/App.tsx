import { createSignal, createEffect, createMemo, onMount, onCleanup, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "tiptap-markdown";
import Placeholder from "@tiptap/extension-placeholder";
import { HashtagHighlight } from "./extensions/hashtag";
import type { NoteInfo, SearchResult } from "./types";
import TopBar from "./components/TopBar";
import BubbleMenu from "./components/BubbleMenu";
import NoteFooter from "./components/NoteFooter";
import StatusBar from "./components/StatusBar";
import DeleteConfirm from "./components/DeleteConfirm";
import Palette from "./components/Palette";
import TagGraph from "./components/TagGraph";
import TagList from "./components/TagList";
import "./App.css";

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
  const [showGraph, setShowGraph] = createSignal(false);
  const [graphVersion, setGraphVersion] = createSignal(0);
  const [graphMode, setGraphMode] = createSignal<"dots" | "list">("dots");
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
  createEffect(() => {
    const t = theme();
    document.documentElement.setAttribute("data-theme", t);
    getCurrentWindow().setTheme(t === "dark" ? "dark" : "light");
  });
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
        HashtagHighlight.configure({ onTagClick: (tag: string) => handleTagClick(tag) }),
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
    clearTimeout(idleTimer);
    idleTimer = setTimeout(() => doAutosave(), AUTOSAVE_IDLE_MS);

    if (charsSinceSave >= AUTOSAVE_CHARS) {
      doAutosave();
      return;
    }

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

  const paletteItemCount = createMemo(() => {
    if (paletteQuery().length > 0) return paletteResults().length;
    return paletteAllNotes().length;
  });

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
    setShowGraph(false);
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
    const q = tag.startsWith("#") ? tag : `#${tag}`;
    setShowPalette(true);
    setPaletteQuery(q);
    setPaletteIndex(0);
    loadPaletteData();
    setTimeout(() => paletteInputRef?.focus(), 30);
  }

  function handleNewNote() {
    if (dirty()) saveNote().then(newBlankNote);
    else newBlankNote();
  }

  // ===== Lifecycle =====
  let unlistenVaultChanged: (() => void) | undefined;

  onMount(() => {
    mountEditor();
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") { e.preventDefault(); saveNote(); }
      else if ((e.metaKey || e.ctrlKey) && e.key === "p") { e.preventDefault(); showPalette() ? closePalette() : openPalette(); }
      else if ((e.metaKey || e.ctrlKey) && e.key === "n") { e.preventDefault(); handleNewNote(); }
    };
    window.addEventListener("keydown", handleKeyDown);

    listen<void>("vault-changed", async () => {
      if (showGraph()) {
        setGraphVersion(v => v + 1);
      } else if (currentNote() && !dirty()) {
        try {
          const content = await invoke<string>("get_note", { id: currentNote()!.id });
          if (editor && !dirty()) {
            const pos = editor.state.selection.anchor;
            editor.commands.setContent(content, false);
            const maxPos = editor.state.doc.content.size;
            editor.commands.setTextSelection(Math.min(pos, maxPos));
          }
          loadNoteDetails(currentNote()!.id);
        } catch (e) {
          console.error("vault-changed:", e);
        }
      }
    }).then(fn => { unlistenVaultChanged = fn; });

    onCleanup(() => {
      window.removeEventListener("keydown", handleKeyDown);
      clearTimeout(idleTimer);
      clearTimeout(maxTimer);
      editor?.destroy();
      unlistenVaultChanged?.();
    });
  });

  let palSearchTimeout: any;
  function handlePaletteInput(val: string) {
    setPaletteQuery(val);
    clearTimeout(palSearchTimeout);
    palSearchTimeout = setTimeout(() => loadPaletteData(), 200);
  }

  const formatDate = createMemo(() => {
    const note = currentNote();
    if (!note?.updated_at) return "";
    try {
      return new Date(note.updated_at).toLocaleDateString("en-US", {
        month: "short", day: "numeric", year: "numeric", hour: "2-digit", minute: "2-digit",
      });
    } catch { return ""; }
  });

  return (
    <div class="app">
      <TopBar
        theme={theme}
        dirty={dirty}
        hasNote={() => !!currentNote()}
        showGraph={showGraph}
        onToggleTheme={toggleTheme}
        onSave={saveNote}
        onDelete={() => setShowDeleteConfirm(true)}
        onOpenPalette={openPalette}
        onNewNote={handleNewNote}
        onToggleGraph={() => setShowGraph(v => !v)}
      />

      <Show when={showGraph()}>
        <div class="graph-view">
          <div class="graph-toolbar">
            <button
              class={`graph-mode-btn${graphMode() === "dots" ? " active" : ""}`}
              onClick={() => setGraphMode("dots")}
            >dots</button>
            <button
              class={`graph-mode-btn${graphMode() === "list" ? " active" : ""}`}
              onClick={() => setGraphMode("list")}
            >list</button>
          </div>
          <Show when={graphMode() === "dots"}>
            <TagGraph onTagClick={handleTagClick} version={graphVersion} />
          </Show>
          <Show when={graphMode() === "list"}>
            <TagList onOpenNote={openNote} />
          </Show>
        </div>
      </Show>

      <div class="canvas" style={{ display: showGraph() ? "none" : undefined }}>
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
            <Show when={bubbleMenu() && editor}>
              <BubbleMenu position={bubbleMenu()!} editor={editor!} />
            </Show>
            <div class="editor-mount" ref={editorEl!} />
          </div>
        </div>
      </div>

      <Show when={currentNote() && (noteTags().length > 0 || relatedNotes().length > 0)}>
        <NoteFooter
          tags={noteTags}
          relatedNotes={relatedNotes}
          onTagClick={handleTagClick}
          onOpenNote={openNote}
        />
      </Show>

      <StatusBar hasNote={() => !!currentNote()} />

      <Show when={showDeleteConfirm()}>
        <DeleteConfirm
          noteTitle={currentNote()?.title || ""}
          onConfirm={() => { deleteNote(); setShowDeleteConfirm(false); }}
          onCancel={() => setShowDeleteConfirm(false)}
        />
      </Show>

      <Show when={showPalette()}>
        <Palette
          query={paletteQuery}
          results={paletteResults}
          allNotes={paletteAllNotes}
          activeIndex={paletteIndex}
          onInput={handlePaletteInput}
          onKeyDown={paletteKeyDown}
          onSelectNote={openNote}
          onNewBlank={() => { newBlankNote(); closePalette(); }}
          onClose={closePalette}
          onHoverIndex={setPaletteIndex}
          inputRef={(el) => { paletteInputRef = el; }}
        />
      </Show>
    </div>
  );
}

export default App;
