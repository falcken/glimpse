import { invoke } from "@tauri-apps/api/core";
import katex from 'katex';
import { v4 as uuidv4 } from 'uuid';

const latexCache: Map<string, string> = new Map();
let activeRenders = 0;
let latexQueueCallback: (() => void) | null = null;

export const resetLatexQueue = () => {
    activeRenders = 0;
    latexQueueCallback = null;
};

export const whenLatexQueueEmpty = (callback: () => void) => {
    if (activeRenders === 0) {
        callback();
    } else {
        latexQueueCallback = callback;
    }
};

const onRenderComplete = () => {
    activeRenders--;
    if (activeRenders === 0 && latexQueueCallback) {
        latexQueueCallback();
    }
}

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

  activeRenders++;
  
  console.log('Rendering LaTeX:', { id, tex });
  const escapedTex = tex.replace(/"/g, '&quot;');

  void callRenderer(id, escapedTex, displayMode);

  return displayMode
    ? blockPlaceholderStyle(id)
    : inlinePlaceholderStyle(calculateWidth(tex), id);
};

const calculateWidth = (tex: string): number => {
    const baseWidth = 10;
    const charWidth = 8; 
    return baseWidth + (tex.length * charWidth);
}

const inlinePlaceholderStyle = (width: number, id: string) => `<span class="latex-placeholder" id="${id}" style="display:inline-block; width:${width}px;"></span>`;
const blockPlaceholderStyle = (id: string) => `<div class="latex-placeholder" id="${id}"></div>`;

const callRenderer = async (id: string, tex: string, displayMode: boolean) => {
    const hash = `${tex}-${displayMode}`;

    if (latexCache.has(hash)) {
        console.log('Using cached LaTeX for', id);
        const svgString = latexCache.get(hash);

        setTimeout(() => {
            replaceWithLatex(id, svgString!, displayMode);
            onRenderComplete();
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
    } finally {
        onRenderComplete();
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