import { invoke } from "@tauri-apps/api/core";
import { v4 as uuidv4 } from 'uuid';

export const renderLatex = (tokens: any[], idx: number, displayMode: boolean): string => {
  const token = tokens[idx];
  const tex = token.content;
  const id = uuidv4();

  const escapedTex = tex.replace(/"/g, '&quot;');
  callRenderer(id, escapedTex, displayMode);

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
  try {
    const svgString = await invoke<string>('render_latex', { tex, displayMode })
    replaceWithLatex(id, svgString, displayMode);
  } catch (error) {
    console.error('Error rendering LaTeX:', error);
  } finally {
    // TODO: should we notify like before?
  }
};

function alignSvgToBaseline(svgElement : SVGSVGElement) {
    if (!svgElement) return;

    // 1. Ensure vertical-align can take effect (ignored on display: block)
    const computedStyle = window.getComputedStyle(svgElement);
    if (computedStyle.display === 'block') {
        svgElement.style.display = 'inline-block';
    }

    // 2. Get viewBox height
    const vb = svgElement.viewBox?.baseVal;
    if (!vb || vb.height === 0) return;

    // 3. Find target rectangle (case-insensitive hex match)
    const rect: SVGRectElement | null = svgElement.querySelector('rect[fill="#f4f4f4" i]') || 
                 svgElement.querySelector('rect[fill="#F4F4F4"]');
    if (!rect) return;

    // 4. Get exact geometry using getBBox() (handles transforms and missing attributes)
    const rectBox = rect.getBBox();
    const rectBottom = rectBox.y + rectBox.height;
    const svgBottom = vb.y + vb.height;
    const gapInUserUnits = svgBottom - rectBottom;

    // 5. Calculate shift in real screen pixels
    const renderedHeight = svgElement.getBoundingClientRect().height;
    if (renderedHeight === 0) return; // Element is hidden or not attached to DOM

    const gapPx = (gapInUserUnits / vb.height) * renderedHeight;

    // 6. Apply pixel offset directly
    svgElement.style.verticalAlign = `-${gapPx}px`;
}

function uniqifySvgUses(svg : SVGSVGElement, id: string) {
  const defs = svg.querySelector("defs")
  if (!defs) return
  for (const elm of defs.children) {
    elm.id = `${id}-${elm.id}`
  }
  for (const use of svg.querySelectorAll("use")) {
    let attr = use.getAttribute("xlink:href")
    if (!attr) continue
    attr = attr.substring(1, attr.length) // remove intial '#'
    use.setAttribute("xlink:href", `#${id}-${attr}`)
  }
}

const replaceWithLatex = (id: string, svgString: string, displayMode: boolean) => {
  const placeholder = document.getElementById(id);
  if (placeholder) {
    placeholder.innerHTML = svgString;
    const svg = placeholder.querySelector('svg')! 
    alignSvgToBaseline(svg)
    placeholder.classList.remove('latex-placeholder');
    placeholder.classList.add(displayMode ? 'latex-rendered-block' : 'latex-rendered-inline');
    placeholder.style.width = ""
    uniqifySvgUses(svg, id)
  }
}
