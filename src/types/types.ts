export interface MarkdownUpdateEvent {
  fileName: string;
  content: string;
  cursorLine: number;
}