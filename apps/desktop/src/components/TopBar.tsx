import { Show, createSignal } from "solid-js";

type Props = {
  theme: () => "light" | "dark";
  dirty: () => boolean;
  hasNote: () => boolean;
  showGraph: () => boolean;
  onToggleTheme: () => void;
  onSave: () => void;
  onDelete: () => void;
  onOpenPalette: () => void;
  onNewNote: () => void;
  onToggleGraph: () => void;
};

export default function TopBar(props: Props) {
  const [hovered, setHovered] = createSignal(false);
  const mod = navigator.platform.includes("Mac") ? "\u2318+" : "Ctrl+";

  return (
    <div class="topbar">
      <div class="topbar-left">
        <span
          class="brand"
          onMouseEnter={() => setHovered(true)}
          onMouseLeave={() => setHovered(false)}
        >
          {hovered() ? `v${__APP_VERSION__}` : "mem"}
        </span>
      </div>
      <div class="topbar-right">
        <button class="topbar-btn theme-toggle" onClick={props.onToggleTheme} title="Toggle theme">
          {props.theme() === "light" ? "\u263E" : "\u2600"}
        </button>
        <Show when={props.hasNote()}>
          <button class="topbar-btn topbar-btn-dim" onClick={props.onDelete} style={{ color: "var(--danger)" }} title="Delete note">
            {"\u2715"}
          </button>
        </Show>
        <button
          class="topbar-btn topbar-btn-dim"
          classList={{ "topbar-btn-active": props.showGraph() }}
          onClick={props.onToggleGraph}
          title="Tag graph"
        >
          Graph
        </button>
        <Show when={props.dirty()}>
          <button class="topbar-btn" onClick={props.onSave} title={`Save (${mod}S)`}>
            Save <div class="save-dot" />
          </button>
        </Show>
        <button class="topbar-btn topbar-btn-dim" onClick={props.onOpenPalette} title={`Search notes (${mod}P)`}>
          Notes <kbd>{mod}P</kbd>
        </button>
        <button class="topbar-btn topbar-btn-dim" onClick={props.onNewNote} title={`New note (${mod}N)`}>
          New <kbd>{mod}N</kbd>
        </button>
      </div>
    </div>
  );
}
