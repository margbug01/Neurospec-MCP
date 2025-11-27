declare module '@tauri-apps/plugin-shell' {
  export function open(url: string): Promise<void>
}
