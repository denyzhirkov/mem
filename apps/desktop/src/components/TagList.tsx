import { createSignal, onMount, For, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { NoteInfo } from "../types";

type TagEntry = { name: string; count: number };

type Props = {
  onOpenNote: (note: NoteInfo) => void;
};

export default function TagList(props: Props) {
  const [allTags, setAllTags] = createSignal<TagEntry[]>([]);
  const [selectedTag, setSelectedTag] = createSignal<string | null>(null);
  const [history, setHistory] = createSignal<string[]>([]);
  const [notes, setNotes] = createSignal<NoteInfo[]>([]);
  const [relatedTags, setRelatedTags] = createSignal<TagEntry[]>([]);

  onMount(async () => {
    const data = await invoke<[string, number][]>("all_tags");
    setAllTags(
      data
        .map(([name, count]) => ({ name, count }))
        .sort((a, b) => b.count - a.count || a.name.localeCompare(b.name))
    );
  });

  async function openTag(tag: string, pushHistory = true) {
    if (pushHistory && selectedTag()) {
      setHistory(h => [...h, selectedTag()!]);
    }
    setSelectedTag(tag);
    const [noteList, related] = await Promise.all([
      invoke<NoteInfo[]>("list_notes_by_tag", { tag }),
      invoke<[string, number][]>("related_tags", { tag }),
    ]);
    setNotes(noteList);
    setRelatedTags(related.map(([name, count]) => ({ name, count })));
  }

  function goBack() {
    const h = history();
    if (h.length === 0) {
      setSelectedTag(null);
      setNotes([]);
      setRelatedTags([]);
      return;
    }
    const prev = h[h.length - 1];
    setHistory(h => h.slice(0, -1));
    openTag(prev, false);
  }

  return (
    <div class="tag-list">
      <Show
        when={selectedTag()}
        fallback={
          <div class="tag-list-all">
            <For each={allTags()}>
              {tag => (
                <button class="tag-list-row" onClick={() => openTag(tag.name)}>
                  <span class="tag-list-name">#{tag.name}</span>
                  <span class="tag-list-count">{tag.count}</span>
                </button>
              )}
            </For>
          </div>
        }
      >
        <div class="tag-list-detail">
          <div class="tag-list-header">
            <button class="tag-list-back" onClick={goBack}>←</button>
            <span class="tag-list-title">#{selectedTag()}</span>
            <Show when={history().length > 0}>
              <span class="tag-list-breadcrumb">
                {history().map(t => `#${t}`).join(" › ")} ›
              </span>
            </Show>
          </div>

          <div class="tag-list-section">
            <div class="tag-list-section-label">notes</div>
            <For each={notes()}>
              {note => (
                <button class="tag-list-note" onClick={() => props.onOpenNote(note)}>
                  {note.title}
                </button>
              )}
            </For>
            <Show when={notes().length === 0}>
              <div class="tag-list-empty">no notes</div>
            </Show>
          </div>

          <Show when={relatedTags().length > 0}>
            <div class="tag-list-section">
              <div class="tag-list-section-label">related tags</div>
              <div class="tag-list-related">
                <For each={relatedTags()}>
                  {tag => (
                    <button class="tag-chip" onClick={() => openTag(tag.name)}>
                      #{tag.name}<span class="tag-chip-count">{tag.count}</span>
                    </button>
                  )}
                </For>
              </div>
            </div>
          </Show>
        </div>
      </Show>
    </div>
  );
}
