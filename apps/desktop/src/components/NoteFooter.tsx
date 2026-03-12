import { Show, For } from "solid-js";
import type { NoteInfo } from "../types";

type Props = {
  tags: () => string[];
  relatedNotes: () => NoteInfo[];
  onTagClick: (tag: string) => void;
  onOpenNote: (note: NoteInfo) => void;
};

export default function NoteFooter(props: Props) {
  return (
    <div class="note-footer">
      <Show when={props.tags().length > 0}>
        <div class="footer-section">
          <div class="footer-label">Tags</div>
          <div class="tag-list">
            <For each={props.tags()}>
              {(tag) => <button class="tag" onClick={() => props.onTagClick(tag)}>#{tag}</button>}
            </For>
          </div>
        </div>
      </Show>
      <Show when={props.relatedNotes().length > 0}>
        <div class="footer-section">
          <div class="footer-label">Related</div>
          <For each={props.relatedNotes()}>
            {(rn) => (
              <div class="related-item" onClick={() => props.onOpenNote(rn)}>
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
  );
}
