// src/menu/menu.ts
import { Menu, Submenu, PredefinedMenuItem } from '@tauri-apps/api/menu';
import { exit } from '@tauri-apps/plugin-process';

export async function setupMenu() {
  const fileSubmenu = await Submenu.new({
    text: 'Glimpse',
    items: [
      {
        id: 'about',
        text: 'About Glimpse',
      },
        await PredefinedMenuItem.new({ item: 'Separator' }),
      {
        id: 'settings',
        accelerator: 'CmdOrCtrl+,',
        text: 'Settings...',
        action: () => {
          console.log('settings pressed');
        },
      },
        await PredefinedMenuItem.new({ item: 'Separator' }),
     {
        id: 'quit',
        text: 'Quit Glimpse',
        accelerator: 'CmdOrCtrl+Q',
        action: () => {
          console.log('quit pressed');
          exit(0); 
        },
      },
    ],
  });

  const menu = await Menu.new({
    items: [
        fileSubmenu,
    ],
  });

  await menu.setAsAppMenu();
}