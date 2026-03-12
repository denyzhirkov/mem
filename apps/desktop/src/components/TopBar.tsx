import { Show, createSignal } from "solid-js";

type Props = {
  theme: () => "light" | "dark";
  dirty: () => boolean;
  hasNote: () => boolean;
  onToggleTheme: () => void;
  onSave: () => void;
  onDelete: () => void;
  onOpenPalette: () => void;
  onNewNote: () => void;
};

export default function TopBar(props: Props) {
  const [hovered, setHovered] = createSignal(false);

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
        <button class="topbar-btn theme-toggle" onClick={props.onToggleTheme}>
          {props.theme() === "light" ? "\u263E" : "\u2600"}
        </button>
        <Show when={props.dirty()}>
          <button class="topbar-btn" onClick={props.onSave}>
            Save <div class="save-dot" />
          </button>
        </Show>
        <Show when={props.hasNote()}>
          <button class="topbar-btn" onClick={props.onDelete} style={{ color: "var(--danger)" }}>
            {"\u2715"}
          </button>
        </Show>
        <button class="topbar-btn" onClick={props.onOpenPalette}>
          Notes <kbd>{"\u2318"}P</kbd>
        </button>
        <button class="topbar-btn" onClick={props.onNewNote}>
          New <kbd>{"\u2318"}N</kbd>
        </button>
      </div>
    </div>
  );
}
