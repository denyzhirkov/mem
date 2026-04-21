import { onMount, onCleanup, createEffect } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import type { TagGraphData } from "../types";

type SimNode = {
  name: string;
  count: number;
  x: number;
  y: number;
  vx: number;
  vy: number;
};

type SimEdge = {
  source: number;
  target: number;
  weight: number;
};

type Props = {
  onTagClick: (tag: string) => void;
  version: () => number;
};

export default function TagGraph(props: Props) {
  let canvas: HTMLCanvasElement | undefined;
  let animId = 0;
  let nodes: SimNode[] = [];
  let edges: SimEdge[] = [];
  let dragging: SimNode | null = null;
  let didDrag = false;
  let offsetX = 0;
  let offsetY = 0;
  let hoveredNode: SimNode | null = null;
  let dpr = 1;
  let lw = 0;
  let lh = 0;

  async function loadData() {
    try {
      const data = await invoke<TagGraphData>("tag_graph");
      const nameToIdx = new Map<string, number>();
      nodes = data.nodes.map((n, i) => {
        nameToIdx.set(n.name, i);
        const angle = (i / data.nodes.length) * Math.PI * 2;
        const r = Math.min(lw, lh) * 0.3;
        return {
          name: n.name,
          count: n.count,
          x: lw / 2 + Math.cos(angle) * r + (Math.random() - 0.5) * 40,
          y: lh / 2 + Math.sin(angle) * r + (Math.random() - 0.5) * 40,
          vx: 0,
          vy: 0,
        };
      });
      edges = data.edges
        .map(e => ({
          source: nameToIdx.get(e.source) ?? -1,
          target: nameToIdx.get(e.target) ?? -1,
          weight: e.weight,
        }))
        .filter(e => e.source >= 0 && e.target >= 0);
    } catch (e) {
      console.error("tag_graph:", e);
    }
  }

  function isSettled(): boolean {
    return nodes.every(n => Math.abs(n.vx) < 0.05 && Math.abs(n.vy) < 0.05);
  }

  function simulate() {
    const cx = lw / 2;
    const cy = lh / 2;
    for (const node of nodes) {
      if (node === dragging) continue;
      node.vx += (cx - node.x) * 0.001;
      node.vy += (cy - node.y) * 0.001;
      for (const other of nodes) {
        if (node === other) continue;
        const dx = node.x - other.x;
        const dy = node.y - other.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = 800 / (dist * dist);
        node.vx += (dx / dist) * force;
        node.vy += (dy / dist) * force;
      }
    }
    for (const edge of edges) {
      const a = nodes[edge.source];
      const b = nodes[edge.target];
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      const dist = Math.sqrt(dx * dx + dy * dy) || 1;
      const force = (dist - 100) * 0.005 * edge.weight;
      if (a !== dragging) { a.vx += (dx / dist) * force; a.vy += (dy / dist) * force; }
      if (b !== dragging) { b.vx -= (dx / dist) * force; b.vy -= (dy / dist) * force; }
    }
    for (const node of nodes) {
      if (node === dragging) continue;
      node.vx *= 0.78;
      node.vy *= 0.78;
      node.x += node.vx;
      node.y += node.vy;
    }
  }

  function nodeRadius(count: number): number {
    return Math.max(6, Math.min(24, 4 + count * 3));
  }

  function draw() {
    const ctx = canvas!.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, lw, lh);

    const style = getComputedStyle(document.documentElement);
    const accent = style.getPropertyValue("--accent").trim();
    const textColor = style.getPropertyValue("--text").trim();
    const dimColor = style.getPropertyValue("--text-dim").trim();
    const tagBg = style.getPropertyValue("--bg-tag").trim();
    const tagColor = style.getPropertyValue("--text-tag").trim();

    ctx.lineWidth = 1;
    for (const edge of edges) {
      const a = nodes[edge.source];
      const b = nodes[edge.target];
      ctx.strokeStyle = dimColor;
      ctx.globalAlpha = Math.min(0.15 + edge.weight * 0.1, 0.5);
      ctx.beginPath();
      ctx.moveTo(a.x, a.y);
      ctx.lineTo(b.x, b.y);
      ctx.stroke();
    }
    ctx.globalAlpha = 1;

    for (const node of nodes) {
      const r = nodeRadius(node.count);
      const isHovered = node === hoveredNode;
      ctx.beginPath();
      ctx.arc(node.x, node.y, r, 0, Math.PI * 2);
      ctx.fillStyle = isHovered ? accent : tagBg;
      ctx.fill();
      ctx.font = `${isHovered ? "600" : "500"} ${isHovered ? 13 : 11}px Inter, system-ui, sans-serif`;
      ctx.fillStyle = isHovered ? textColor : tagColor;
      ctx.textAlign = "center";
      ctx.fillText(`#${node.name}`, node.x, node.y + r + 14);
      if (node.count > 1) {
        ctx.font = "600 9px Inter, system-ui, sans-serif";
        ctx.fillStyle = isHovered ? textColor : tagColor;
        ctx.fillText(String(node.count), node.x, node.y + 3);
      }
    }
  }

  function loop() {
    simulate();
    draw();
    if (isSettled() && !dragging) {
      animId = 0;
      return;
    }
    animId = requestAnimationFrame(loop);
  }

  function startLoop() {
    cancelAnimationFrame(animId);
    animId = requestAnimationFrame(loop);
  }

  function findNode(x: number, y: number): SimNode | null {
    for (const node of nodes) {
      const r = nodeRadius(node.count) + 4;
      const dx = x - node.x;
      const dy = y - node.y;
      if (dx * dx + dy * dy < r * r) return node;
    }
    return null;
  }

  function handleMouseDown(e: MouseEvent) {
    const rect = canvas!.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const node = findNode(x, y);
    if (node) {
      dragging = node;
      didDrag = false;
      offsetX = x - node.x;
      offsetY = y - node.y;
      startLoop();
    }
  }

  function handleMouseMove(e: MouseEvent) {
    const rect = canvas!.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    if (dragging) {
      dragging.x = x - offsetX;
      dragging.y = y - offsetY;
      dragging.vx = 0;
      dragging.vy = 0;
      didDrag = true;
    }
    const prev = hoveredNode;
    hoveredNode = findNode(x, y);
    canvas!.style.cursor = hoveredNode ? "pointer" : "default";
    if (prev !== hoveredNode && !animId) draw();
  }

  function handleMouseUp() {
    dragging = null;
  }

  function handleClick(e: MouseEvent) {
    if (didDrag) { didDrag = false; return; }
    const rect = canvas!.getBoundingClientRect();
    const node = findNode(e.clientX - rect.left, e.clientY - rect.top);
    if (node) props.onTagClick(node.name);
  }

  function resize() {
    if (!canvas) return;
    dpr = window.devicePixelRatio || 1;
    lw = canvas.parentElement!.clientWidth;
    lh = canvas.parentElement!.clientHeight;
    canvas.width = lw * dpr;
    canvas.height = lh * dpr;
    canvas.style.width = `${lw}px`;
    canvas.style.height = `${lh}px`;
  }

  createEffect(async () => {
    const v = props.version();
    if (v > 0) {
      await loadData();
      startLoop();
    }
  });

  onMount(async () => {
    resize();
    window.addEventListener("resize", resize);
    await loadData();
    startLoop();
  });

  onCleanup(() => {
    cancelAnimationFrame(animId);
    window.removeEventListener("resize", resize);
  });

  return (
    <div class="tag-graph">
      <canvas
        ref={canvas!}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onClick={handleClick}
      />
    </div>
  );
}
