import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

export type HashtagOptions = {
  onTagClick: (tag: string) => void;
};

const TAG_REGEX = /(?:^|\s)#([a-zA-Z0-9_-]+)/g;

const hashtagPluginKey = new PluginKey("hashtagHighlight");

export const HashtagHighlight = Extension.create<HashtagOptions>({
  name: "hashtagHighlight",

  addOptions() {
    return { onTagClick: () => {} };
  },

  addProseMirrorPlugins() {
    const onTagClick = this.options.onTagClick;

    return [
      new Plugin({
        key: hashtagPluginKey,
        props: {
          decorations(state) {
            const decorations: Decoration[] = [];
            state.doc.descendants((node, pos) => {
              if (!node.isText || !node.text) return;
              const text = node.text;
              let match: RegExpExecArray | null;
              TAG_REGEX.lastIndex = 0;
              while ((match = TAG_REGEX.exec(text)) !== null) {
                // match[0] might include leading space, match[1] is the tag name
                const fullMatch = match[0];
                const tagName = match[1];
                // Calculate position of "#tagname" (skip leading space if any)
                const hashStart = match.index + (fullMatch.length - tagName.length - 1);
                const from = pos + hashStart;
                const to = from + tagName.length + 1; // +1 for the #
                decorations.push(
                  Decoration.inline(from, to, {
                    class: "editor-tag",
                    nodeName: "span",
                    "data-tag": tagName,
                  })
                );
              }
            });
            return DecorationSet.create(state.doc, decorations);
          },
          handleClick(_view, _pos, event) {
            const target = event.target as HTMLElement;
            if (target.classList?.contains("editor-tag")) {
              const tag = target.getAttribute("data-tag");
              if (tag) {
                event.preventDefault();
                onTagClick(tag);
                return true;
              }
            }
            return false;
          },
        },
      }),
    ];
  },
});
