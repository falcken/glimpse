import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

import "katex/dist/katex.min.css";
import MarkdownIt from "markdown-it";
import mdLineNumbers from "markdown-it-inject-linenumbers";
import { renderLatex, renderInlineKatex, resetLatexQueue, whenLatexQueueEmpty } from "./latex/render";

import { setupMenu } from "./menu/menu";
import { SettingsManager } from './settings/settings';

import texmath from "markdown-it-texmath";

import { MarkdownUpdateEvent } from "./types/types";

// Initialize MarkdownIt with plugins
const md = new MarkdownIt().use(mdLineNumbers).use(texmath, {
  delimiters: "dollars",
});

md.renderer.rules.math_inline = (tokens, idx): string => {
  return renderInlineKatex(tokens, idx, false);
};

md.renderer.rules.math_block = (tokens, idx): string => {
  return renderLatex(tokens, idx, true);
};

md.renderer.rules.math_inline_double = md.renderer.rules.math_block;
md.renderer.rules.math_block_eqno = md.renderer.rules.math_block;

// Render Markdown on events
const contentEl = document.getElementById("content");

let lastContent = "";
let lastCursorLine = 0;

listen<MarkdownUpdateEvent>("markdown-update", (event) => {
  const { fileName, content, cursorLine } = event.payload;

  lastContent = content;
  lastCursorLine = cursorLine;

  resetLatexQueue();

  updateFileName(fileName);
  renderMarkdown(content);
  scrollIntoView(cursorLine);

  whenLatexQueueEmpty(() => {
    requestAnimationFrame(() => {
      scrollIntoView(cursorLine);
    });
  });
});

window.addEventListener("settings-changed", () => {
  resetLatexQueue();
  renderMarkdown(lastContent);

  whenLatexQueueEmpty(() => {
    requestAnimationFrame(() => {
      scrollIntoView(lastCursorLine);
    });
  });
});

const updateFileName = (fileName: string) => {
  const parts = fileName.split(/[/\\]/);
  const shortName = parts[parts.length - 1];

  const titleEl = document.getElementById("file-name");
  if (titleEl) titleEl.textContent = shortName;
};

const renderMarkdown = (markdown: string) => {
  const html = md.render(markdown);
  if (contentEl) contentEl.innerHTML = html;
};

const highlightScrolledLine = (lineNumber: number) => {
  const previouslyHighlighted = document.querySelectorAll(
    ".line-highlighted"
  );
  previouslyHighlighted.forEach((el) => {
    el.classList.remove("line-highlighted");
  });

  const newHighlight = document.querySelector(
    `[data-source-line="${lineNumber}"]`
  );
  if (newHighlight) {
    newHighlight.classList.add("line-highlighted");
  }
}

const scrollIntoView = (lineNumber: number) => {
  let targetLine = lineNumber;
  let targetElement: HTMLElement | null = null;

  while (targetLine >= 0 && !targetElement) {
    targetElement = document.querySelector(
      `[data-source-line="${targetLine}"]`
    );
    if (!targetElement) targetLine--;
  }

  if (targetElement) {
    const opts: ScrollIntoViewOptions = { behavior: "instant" };

    // targetLine < lineNumber ? (opts.block = "start") : (opts.block = "end");
    opts.block = "center";

    targetElement.scrollIntoView(opts);
    highlightScrolledLine(targetLine);

    console.log(`Scrolled to line ${targetLine}`);
  } else {
    console.warn(`No element found for cursor line ${lineNumber}`);
  }
};

const handleCmdClick = (event: MouseEvent) => {
  if (!(event.metaKey || event.ctrlKey)) return;

  const target = event.target as HTMLElement;
  const lineAttr = target.getAttribute("data-source-line");
  if (lineAttr) {
    let lineNumber = parseInt(lineAttr, 10);
    lineNumber = lineNumber > 1 ? lineNumber + 1 : 1;
    invoke("line_clicked", { lineNumber });
  }
};

const initApp = () => {
  // CMD+Click handling
  document.addEventListener("click", handleCmdClick);

  // Setup menu
  setupMenu().catch(console.error);

  // Settings
  new SettingsManager();
};


// Setup application menu
initApp();