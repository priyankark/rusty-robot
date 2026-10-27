/** Source-built Node-API bridge. Media promises can reject when the two-job queue is full. */
export interface Frame {
  data?: Buffer | null;
  width: number; height: number; sourceWidth: number; sourceHeight: number;
  fingerprint: string; changed: boolean;
  captureMs: number; convertMs: number; resizeMs: number; hashMs: number; encodeMs: number;
}
export function captureFrame(width?: number | null, quality?: number | null, previous?: string | null, force?: boolean, format?: 'jpeg' | 'jpg' | 'png'): Promise<Frame>;
export function resizeImage(input: Buffer, width: number, height?: number | null, quality?: number | null, format?: 'jpeg' | 'jpg' | 'png'): Promise<Frame>;
export function getScreenSize(): { width: number; height: number };
export function getMousePos(): { x: number; y: number };
export function getPermissions(): { accessibility?: boolean | null; screenCapture?: boolean | null };
export function moveMouse(x: number, y: number): void;
export function dragMouse(x: number, y: number, button: string): void;
export function mouseToggle(state: string, button: string): void;
export function mouseClick(button?: string): void;
export function scrollMouse(x: number, y: number): void;
export function keyTap(key: string, modifiers?: string[]): void;
export function keyToggle(key: string, state: string, modifiers?: string[]): void;
export function typeString(text: string): void;
export function setKeyboardDelay(ms: number): void;
