type Props = {
  hasNote: () => boolean;
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
      <span>{props.hasNote() ? "Saved" : "New note"}</span>
    </div>
  );
}
