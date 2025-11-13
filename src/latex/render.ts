import { invoke } from "@tauri-apps/api/core";
import katex from 'katex';
import { v4 as uuidv4 } from 'uuid';

const latexCache: Map<string, string> = new Map();

export const renderInlineKatex = (tokens: any[], idx: number, displayMode: boolean): string => {
  const token = tokens[idx];
  const tex = token.content;

  try {
    return katex.renderToString(tex, {
      throwOnError: false,
      displayMode: displayMode
    });
  } catch (error) {
    console.error("KaTeX Error:", error);
    return `<span class="katex-error">${tex}</span>`;
  }
};

// Keep in mind that inline is handled by KaTeX right now.
export const renderLatex = (tokens: any[], idx: number, displayMode: boolean): string => {
  const token = tokens[idx];
  const tex = token.content;
  const id = uuidv4();  

  console.log('Rendering LaTeX:', { id, tex });
  const escapedTex = tex.replace(/"/g, '&quot;');

  void callRenderer(id, escapedTex, displayMode);

  return displayMode
    ? blockPlaceholderStyle(calculateHeight(tex), id)
    : inlinePlaceholderStyle(calculateWidth(tex), id);
}

const calculateWidth = (tex: string): number => {
    const baseWidth = 10;
    const charWidth = 8; 
    return baseWidth + (tex.length * charWidth);
}

const calculateHeight = (tex: string): number => {
    return 50;
}

const inlinePlaceholderStyle = (width: number, id: string) => `<span class="latex-placeholder" id="${id}" style="display:inline-block; width:${width}px;"></span>`;
const blockPlaceholderStyle = (height: number, id: string) => `<div class="latex-placeholder" id="${id}" style="display:block; height:${height}px; margin: 10px 0;"></div>`;

const callRenderer = async (id: string, tex: string, displayMode: boolean) => {
    const hash = `${tex}-${displayMode}`;

    if (latexCache.has(hash)) {
        console.log('Using cached LaTeX for', id);
        const svgString = latexCache.get(hash);

        setTimeout(() => {
            replaceWithLatex(id, svgString!, displayMode);
        }, 0);

        return;
    }

    try {
        const svgString = await invoke<string>('render_latex', { id, tex, displayMode });
        console.log('Received rendered SVG for', id);

        replaceWithLatex(id, svgString, displayMode);
        
        latexCache.set(hash, svgString);
    } catch (error) {
        console.error('Error rendering LaTeX:', error);
    }
};

const replaceWithLatex = (id: string, svgString: string, displayMode: boolean) => {
    const placeholder = document.getElementById(id);
    if (placeholder) {
        placeholder.innerHTML = svgString;
        placeholder.classList.remove('latex-placeholder');
        placeholder.classList.add(displayMode ? 'latex-rendered-block' : 'latex-rendered-inline');
    }
}