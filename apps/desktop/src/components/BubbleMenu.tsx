import type { Editor } from "@tiptap/core";

type Props = {
  position: { top: number; left: number };
  editor: Editor;
};

export default function BubbleMenu(props: Props) {
  const toggle = (e: MouseEvent, action: () => void) => {
    e.preventDefault();
    action();
  };

  return (
    <div class="bubble-menu" style={{ top: `${props.position.top}px`, left: `${props.position.left}px` }}>
      <button class="bubble-btn" onMouseDown={(e) => toggle(e, () => props.editor.chain().focus().toggleBold().run())} classList={{ active: props.editor.isActive("bold") }}>B</button>
      <button class="bubble-btn" onMouseDown={(e) => toggle(e, () => props.editor.chain().focus().toggleItalic().run())} classList={{ active: props.editor.isActive("italic") }}><em>I</em></button>
      <button class="bubble-btn" onMouseDown={(e) => toggle(e, () => props.editor.chain().focus().toggleStrike().run())} classList={{ active: props.editor.isActive("strike") }}><s>S</s></button>
      <button class="bubble-btn" onMouseDown={(e) => toggle(e, () => props.editor.chain().focus().toggleCode().run())} classList={{ active: props.editor.isActive("code") }}>&lt;/&gt;</button>
    </div>
  );
}
