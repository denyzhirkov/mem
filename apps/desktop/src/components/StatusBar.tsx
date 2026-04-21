import { Show } from "solid-js";

type Props = {
  hasNote: () => boolean;
  updateAvailable: () => string | null;
};

export default function StatusBar(props: Props) {
  return (
    <div class="statusbar">
      <div class="statusbar-hints">
        <span># heading</span>
        <span>**bold**</span>
        <span>*italic*</span>
        <span>- list</span>
        <span>&gt; quote</span>
        <span>#tag</span>
      </div>
      <Show when={props.updateAvailable()} fallback={
        <span>{props.hasNote() ? "Saved" : "New note"}</span>
      }>
        <a
          class="statusbar-update"
          href={`https://github.com/denyzhirkov/mem/releases/tag/v${props.updateAvailable()}`}
          target="_blank"
          rel="noreferrer"
        >
          ↑ v{props.updateAvailable()} available
        </a>
      </Show>
    </div>
  );
}
