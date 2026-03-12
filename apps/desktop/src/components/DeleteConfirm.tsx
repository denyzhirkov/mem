type Props = {
  noteTitle: string;
  onConfirm: () => void;
  onCancel: () => void;
};

export default function DeleteConfirm(props: Props) {
  return (
    <div class="delete-bar">
      <span>Delete "{props.noteTitle || "this note"}"?</span>
      <button class="btn-sm btn-danger" onClick={props.onConfirm}>Delete</button>
      <button class="btn-sm btn-cancel" onClick={props.onCancel}>Cancel</button>
    </div>
  );
}
