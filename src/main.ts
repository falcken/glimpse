import { listen } from '@tauri-apps/api/event';
import { invoke } from "@tauri-apps/api/core";

import MarkdownIt from 'markdown-it';
import mdLineNumbers from 'markdown-it-inject-linenumbers';

interface MarkdownUpdateEvent {
  fileName: string;
  content: string;
  cursorLine: number;
}

const md = new MarkdownIt()
  .use(mdLineNumbers)

const contentEl = document.getElementById('content');

listen<MarkdownUpdateEvent>('markdown-update', (event) => {
  const { fileName, content, cursorLine } = event.payload;
  updateFileName(fileName);
  renderMarkdown(content);
  scrollIntoView(cursorLine);
});

const updateFileName = (fileName: string) => {
  const parts = fileName.split(/[/\\]/);
  const shortName = parts[parts.length - 1];


  const titleEl = document.getElementById('file-name');
  if (titleEl) titleEl.textContent = shortName;
}

const renderMarkdown = (markdown: string) => {
  const html = md.render(markdown);
  if (contentEl) contentEl.innerHTML = html;
}

const scrollIntoView = (lineNumber: number) => {
  let targetLine = lineNumber;
  let targetElement: HTMLElement | null = null;

  while (targetLine > 0 && !targetElement) {
    targetElement = document.querySelector(`[data-source-line="${targetLine}"]`);
    targetLine--;
  }

  if (targetElement) {
    targetElement.scrollIntoView({ behavior: 'auto', block: 'center' });
  } else {
    console.warn(`No element found for cursor line ${lineNumber}`);
  }
}

const handleCmdClick = (event: MouseEvent) => {
  if (!(event.metaKey || event.ctrlKey)) return;

  const target = event.target as HTMLElement;
  const lineAttr = target.getAttribute('data-source-line');
  if (lineAttr) {
    let lineNumber = parseInt(lineAttr, 10);
    lineNumber = lineNumber > 1 ? lineNumber + 1 : 1;
    invoke('line_clicked', { lineNumber } );
  }
}

document.addEventListener('click', handleCmdClick);