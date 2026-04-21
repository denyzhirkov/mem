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

  async function openTag(tag: string, prevTag: string | null = null, newHistory?: string[]) {
    if (newHistory !== undefined) {
      setHistory(newHistory);
    } else if (prevTag) {
      setHistory(h => [...h, prevTag]);
    }
    setSelectedTag(tag);
    const [noteList, related] = await Promise.all([
      invoke<NoteInfo[]>("list_notes_by_tag", { tag }),
      invoke<[string, number][]>("related_tags", { tag }),
    ]);
    setNotes(noteList);

    // Move the tag we came from to the end
    let list = related.map(([name, count]) => ({ name, count }));
    if (prevTag) {
      const idx = list.findIndex(t => t.name === prevTag);
      if (idx >= 0) list = [...list.slice(0, idx), ...list.slice(idx + 1), list[idx]];
    }
    setRelatedTags(list);
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
    openTag(prev, null, h.slice(0, -1));
  }

  function jumpTo(idx: number) {
    const h = history();
    openTag(h[idx], null, h.slice(0, idx));
  }

  return (
    <div class="tag-list-view">
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
            <div class="tag-list-crumb-trail">
              <For each={history()}>
                {(t, i) => (
                  <>
                    <button class="tag-list-crumb" onClick={() => jumpTo(i())}>#{t}</button>
                    <span class="tag-list-crumb-sep">›</span>
                  </>
                )}
              </For>
              <span class="tag-list-crumb-current">#{selectedTag()}</span>
            </div>
          </div>

          <Show when={relatedTags().length > 0}>
            <div class="tag-list-section">
              <div class="tag-list-section-label">related tags</div>
              <div class="tag-list-related">
                <For each={relatedTags()}>
                  {tag => (
                    <button class="tag-chip" onClick={() => openTag(tag.name, selectedTag())}>
                      #{tag.name}<span class="tag-chip-count">{tag.count}</span>
                    </button>
                  )}
                </For>
              </div>
            </div>
          </Show>

          <div class="tag-list-section">
            <div class="tag-list-section-label">notes</div>
            <div class="tag-list-notes">
              <For each={notes()}>
                {(note, i) => (
                  <button
                    class={`tag-list-note${i() % 2 === 0 ? " tag-list-note-stripe" : ""}`}
                    onClick={() => props.onOpenNote(note)}
                  >
                    {note.title}
                  </button>
                )}
              </For>
              <Show when={notes().length === 0}>
                <div class="tag-list-empty">no notes</div>
              </Show>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}
