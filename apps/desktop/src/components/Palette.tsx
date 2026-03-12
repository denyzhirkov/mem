import { Show, For, createSignal, createMemo } from "solid-js";
import type { NoteInfo, SearchResult } from "../types";

const ROW_HEIGHT = 42;
const ROW_HEIGHT_SEARCH = 60;
const MAX_LIST_HEIGHT = 330;
const OVERSCAN = 3;

type Props = {
  query: () => string;
  results: () => SearchResult[];
  allNotes: () => NoteInfo[];
  activeIndex: () => number;
  onInput: (val: string) => void;
  onKeyDown: (e: KeyboardEvent) => void;
  onSelectNote: (note: NoteInfo) => void;
  onNewBlank: () => void;
  onClose: () => void;
  onHoverIndex: (i: number) => void;
  inputRef: (el: HTMLInputElement) => void;
};

function useVirtualList<T>(items: () => T[], rowHeight: number) {
  const [scrollTop, setScrollTop] = createSignal(0);

  const listHeight = createMemo(() =>
    Math.min(items().length * rowHeight, MAX_LIST_HEIGHT)
  );

  const visible = createMemo(() => {
    const all = items();
    const top = scrollTop();
    const viewH = listHeight();
    const first = Math.max(0, Math.floor(top / rowHeight) - OVERSCAN);
    const last = Math.min(all.length, Math.ceil((top + viewH) / rowHeight) + OVERSCAN);
    return {
      containerHeight: all.length * rowHeight,
      viewerTop: first * rowHeight,
      items: all.slice(first, last),
      firstIdx: first,
    };
  });

  const onScroll = (e: Event) => {
    const target = e.target as HTMLElement;
    setScrollTop(target.scrollTop);
  };

  return { visible, listHeight, onScroll };
}

function ExcerptHtml(props: { text: string; kind: string }) {
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

function noteFromResult(r: SearchResult): NoteInfo {
  return { id: r.note_id, title: r.title, slug: "", tags: [], updated_at: "" };
}

export default function Palette(props: Props) {
  const isSearchMode = () => props.query().length > 0;

  const browse = useVirtualList(() => props.allNotes(), ROW_HEIGHT);
  const search = useVirtualList(() => props.results(), ROW_HEIGHT_SEARCH);

  return (
    <div class="palette-overlay" onClick={(e) => { if (e.target === e.currentTarget) props.onClose(); }}>
      <div class="palette">
        <input
          ref={props.inputRef}
          class="palette-input"
          type="text"
          placeholder="Search notes..."
          value={props.query()}
          onInput={(e) => props.onInput(e.currentTarget.value)}
          onKeyDown={props.onKeyDown}
        />
        <div class="palette-action-wrap">
          <div class="palette-action" onClick={props.onNewBlank}>
            + New blank note
          </div>
        </div>

        {/* Search results mode */}
        <Show when={isSearchMode()}>
          <Show when={props.results().length > 0} fallback={
            <div class="palette-empty">No notes found. Press Enter to create one.</div>
          }>
            <div
              class="palette-list"
              style={{ height: `${search.listHeight()}px` }}
              onScroll={search.onScroll}
            >
              <div style={{ height: `${search.visible().containerHeight}px`, position: "relative" }}>
                <div style={{ position: "absolute", top: `${search.visible().viewerTop}px`, width: "100%" }}>
                  <For each={search.visible().items}>
                    {(result, localIdx) => {
                      const realIdx = () => search.visible().firstIdx + localIdx();
                      return (
                        <div
                          class={`palette-item ${props.activeIndex() === realIdx() ? "active" : ""}`}
                          style={{ height: `${ROW_HEIGHT_SEARCH}px` }}
                          onClick={() => props.onSelectNote(noteFromResult(result))}
                          onMouseEnter={() => props.onHoverIndex(realIdx())}
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
                      );
                    }}
                  </For>
                </div>
              </div>
            </div>
          </Show>
        </Show>

        {/* Browse mode — all notes */}
        <Show when={!isSearchMode()}>
          <div
            class="palette-list"
            style={{ height: `${browse.listHeight()}px` }}
            onScroll={browse.onScroll}
          >
            <div style={{ height: `${browse.visible().containerHeight}px`, position: "relative" }}>
              <div style={{ position: "absolute", top: `${browse.visible().viewerTop}px`, width: "100%" }}>
                <For each={browse.visible().items}>
                  {(note, localIdx) => {
                    const realIdx = () => browse.visible().firstIdx + localIdx();
                    return (
                      <div
                        class={`palette-item ${props.activeIndex() === realIdx() ? "active" : ""}`}
                        style={{ height: `${ROW_HEIGHT}px` }}
                        onClick={() => props.onSelectNote(note)}
                        onMouseEnter={() => props.onHoverIndex(realIdx())}
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
                    );
                  }}
                </For>
              </div>
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
}
