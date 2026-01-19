# Dark Mode Implementation Guide

This document describes how dark mode is implemented in DataSpeak using Tailwind CSS v4, next-themes, and Tauri.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Implementation Details](#implementation-details)
- [Usage](#usage)
- [Troubleshooting](#troubleshooting)
- [How It Works](#how-it-works)

## Overview

DataSpeak uses a modern dark mode implementation optimized for desktop applications built with Tauri. The solution combines:

- **Custom Theme Provider**: Lightweight React context-based theme management
- **Tailwind CSS v4**: Utility-first CSS framework with native dark mode support
- **Tauri Window API**: Native window theme synchronization
- **CSS Custom Properties**: Dynamic theming with CSS variables

## Architecture

### Key Components

```
┌─────────────────────────────────────────────────────────────┐
│                     User Interface                          │
│  ┌──────────────┐  ┌─────────────────┐                     │
│  │   Settings   │  │  Any Component  │                     │
│  │    Dialog    │  │  (uses theme)   │                     │
│  └──────┬───────┘  └────────┬────────┘                     │
│         │                    │                              │
│         └────────────────────┘                              │
│                   │                                         │
│          ┌────────▼────────┐                                │
│          │   useTheme()    │                                │
│          │  (React Hook)   │                                │
│          └────────┬────────┘                                │
│                   │                                         │
│          ┌────────▼────────┐                                │
│          │ ThemeProvider   │                                │
│          │  (React Context)│                                │
│          └────────┬────────┘                                │
│                   │                                         │
├───────────────────┼─────────────────────────────────────────┤
│          Storage & OS Layer                                 │
│    ┌──────────────┼──────────────────┐                     │
│    │              │                  │                     │
│  ┌─▼──────────┐  ┌▼─────────────┐  ┌▼──────────┐          │
│  │localStorage│  │HTML classList│  │   Tauri   │          │
│  │ (persist)  │  │  (apply CSS) │  │ Window API│          │
│  └────────────┘  └──────────────┘  └───────────┘          │
└─────────────────────────────────────────────────────────────┘
```

### File Structure

```
src/
├── components/
│   ├── theme-provider.tsx      # Theme provider with Tauri sync
│   └── settings/
│       └── SettingsDialog.tsx  # Theme selection UI
├── index.css                   # Tailwind v4 + theme variables
└── App.tsx                     # App wrapped with ThemeProvider

index.html                      # FOUC prevention script
```

## Implementation Details

### 1. CSS Theme Variables (`src/index.css`)

The CSS uses a clean structure optimized for Tailwind CSS v4:

```css
@import "tailwindcss";
@plugin "tailwindcss-animate";

/* Light mode (default) */
:root {
  --radius: 0.625rem;
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.145 0 0);
  /* ... more variables */
}

/* Dark mode */
.dark {
  --background: oklch(0.145 0 0);
  --foreground: oklch(0.985 0 0);
  --card: oklch(0.205 0 0);
  --card-foreground: oklch(0.985 0 0);
  /* ... more variables */
}

/* Map CSS variables to Tailwind theme */
@theme {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  /* ... more mappings */
}
```

**Key Points:**

- **`:root`** defines light mode variables (default)
- **`.dark`** overrides variables for dark mode
- **`@theme`** maps CSS variables to Tailwind utilities
- Uses **OKLCH color space** for better color perception
- All shadcn/ui components automatically respect these variables

### 2. Theme Provider (`src/components/theme-provider.tsx`)

```tsx
import { createContext, useContext, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

type Theme = "dark" | "light" | "system";

type ThemeProviderProps = {
  children: React.ReactNode;
  defaultTheme?: Theme;
  storageKey?: string;
};

type ThemeProviderState = {
  theme: Theme;
  setTheme: (theme: Theme) => void;
};

const initialState: ThemeProviderState = {
  theme: "system",
  setTheme: () => null,
};

const ThemeProviderContext = createContext<ThemeProviderState>(initialState);

export function ThemeProvider({
  children,
  defaultTheme = "system",
  storageKey = "vite-ui-theme",
  ...props
}: ThemeProviderProps) {
  const [theme, setTheme] = useState<Theme>(
    () => (localStorage.getItem(storageKey) as Theme) || defaultTheme
  );

  useEffect(() => {
    const root = window.document.documentElement;

    root.classList.remove("light", "dark");

    if (theme === "system") {
      const systemTheme = window.matchMedia("(prefers-color-scheme: dark)")
        .matches
        ? "dark"
        : "light";

      root.classList.add(systemTheme);

      // Sync Tauri window theme
      updateTauriTheme(systemTheme);
      return;
    }

    root.classList.add(theme);

    // Sync Tauri window theme
    updateTauriTheme(theme);
  }, [theme]);

  const value = {
    theme,
    setTheme: (theme: Theme) => {
      localStorage.setItem(storageKey, theme);
      setTheme(theme);
    },
  };

  return (
    <ThemeProviderContext.Provider {...props} value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}

// Helper to update Tauri window theme
async function updateTauriTheme(theme: "dark" | "light") {
  try {
    const appWindow = getCurrentWindow();
    await appWindow.setTheme(theme);
  } catch (error) {
    console.debug("Tauri window theme API not available");
  }
}

export const useTheme = () => {
  const context = useContext(ThemeProviderContext);

  if (context === undefined)
    throw new Error("useTheme must be used within a ThemeProvider");

  return context;
};
```

**Features:**

- Custom React Context-based implementation (no external dependencies)
- Lightweight and optimized for Tauri desktop apps
- Automatic system theme detection using `matchMedia`
- Syncs native window titlebar via Tauri API
- Gracefully handles non-Tauri environments (browser dev)
- Persists theme preference to localStorage
- Based on official shadcn/ui recommendation for Vite apps

### 3. FOUC Prevention Script (`index.html`)

Prevents flash of unstyled content on app load:

```html
<script>
  (function() {
    const storageKey = 'dataspeak-theme';
    const defaultTheme = 'system';

    function getTheme() {
      let theme;
      try {
        theme = localStorage.getItem(storageKey);
      } catch (e) {
        console.warn('localStorage blocked:', e);
      }
      return theme || defaultTheme;
    }

    function getSystemTheme() {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
    }

    const theme = getTheme();
    const resolvedTheme = theme === 'system' ? getSystemTheme() : theme;

    // Remove any existing theme classes first
    document.documentElement.classList.remove('light', 'dark');

    // Add the resolved theme class
    if (resolvedTheme) {
      document.documentElement.classList.add(resolvedTheme);
    }
  })();
</script>
```

**Why This is Needed:**

- Runs **before** React hydrates
- Reads theme from localStorage
- Applies theme class immediately
- Prevents white flash on dark mode
- Essential for desktop apps where users expect instant theming

### 4. App Integration (`src/App.tsx`)

```tsx
import { ThemeProvider } from "@/components/theme-provider";

function App() {
  return (
    <ThemeProvider
      defaultTheme="system"         // Default to OS preference
      storageKey="dataspeak-theme"  // localStorage key
    >
      {/* Your app content */}
    </ThemeProvider>
  );
}
```

### 5. Tauri Configuration (`src-tauri/tauri.conf.json`)

```json
{
  "app": {
    "windows": [
      {
        "title": "DataSpeak",
        "width": 1280,
        "height": 800,
        "theme": null  // null = follow app theme
      }
    ]
  }
}
```

Setting `"theme": null` allows the window theme to be controlled programmatically via `appWindow.setTheme()`.

## Usage

### For Users

1. **Open Settings** (Settings icon in header)
2. **Select Theme**:
   - **Light**: Always light mode
   - **Dark**: Always dark mode
   - **System**: Follow OS theme preference
3. Theme persists across app restarts

### For Developers

#### Using Theme in Components

```tsx
import { useTheme } from "@/components/theme-provider";

function MyComponent() {
  const { theme, setTheme, resolvedTheme } = useTheme();

  return (
    <div>
      <p>Current theme: {theme}</p>
      <p>Resolved theme: {resolvedTheme}</p>

      <button onClick={() => setTheme("dark")}>
        Switch to Dark
      </button>
    </div>
  );
}
```

#### Using Tailwind Dark Mode Classes

```tsx
<div className="bg-white dark:bg-black">
  <h1 className="text-black dark:text-white">
    This text adapts to theme
  </h1>
</div>
```

#### Using CSS Variables Directly

```tsx
<div style={{ backgroundColor: 'var(--background)', color: 'var(--foreground)' }}>
  Uses theme colors
</div>
```

## Troubleshooting

### Theme Not Switching

**Debug Steps:**

1. Check if `dark` class is applied to `<html>`:
   ```js
   console.log(document.documentElement.classList);
   // Should show: DOMTokenList ['dark'] or ['light']
   ```

2. Check localStorage:
   ```js
   localStorage.getItem('dataspeak-theme');
   // Should return: 'light', 'dark', or 'system'
   ```

3. Verify CSS variables:
   ```js
   getComputedStyle(document.documentElement).getPropertyValue('--background');
   // Should return different values for light/dark
   ```

4. Check next-themes state in React DevTools:
   - Install React DevTools extension
   - Find ThemeProvider in component tree
   - Check context values

### Common Issues

#### Issue: Colors don't change despite class switching

**Cause**: Tailwind CSS v4 not compiling theme correctly

**Fix**: Ensure `@theme` directive maps variables correctly:
```css
@theme {
  --color-background: var(--background);
  /* NOT: --color-background: oklch(1 0 0); */
}
```

#### Issue: Flash of white on app start

**Cause**: FOUC prevention script missing or incorrect

**Fix**: Ensure `index.html` has the blocking script in `<head>` before other scripts

#### Issue: System theme not updating

**Cause**: Browser/OS not detecting theme changes

**Fix**: This is a limitation of `matchMedia`. Users should manually refresh or restart the app after OS theme changes.

#### Issue: Titlebar doesn't match content

**Cause**: Tauri theme API not syncing

**Fix**: Check `TauriThemeSync` component is rendering and not throwing errors. View console for:
```
Tauri window theme API not available: [error]
```

### Development Tips

1. **Test all three modes**: Light, Dark, and System
2. **Test OS theme changes**: Change your OS theme while app is running
3. **Test persistence**: Close and reopen app to verify saved preference
4. **Test in production**: `pnpm tauri build` - some issues only appear in builds
5. **Use browser DevTools**: Inspect `<html>` element to verify class changes
6. **Monitor console**: Check for theme-related errors or warnings

## How It Works

### Theme Change Flow

```
User clicks "Dark" in Settings
          ↓
setTheme("dark") called
          ↓
next-themes updates state
          ↓
localStorage.setItem("dataspeak-theme", "dark")
          ↓
React re-renders ThemeProvider
          ↓
document.documentElement.classList updates → ['dark']
          ↓
Tailwind applies dark: variants
          ↓
TauriThemeSync detects change
          ↓
appWindow.setTheme("dark")
          ↓
Native window titlebar updates
```

### System Theme Detection

```
OS theme changes (Light → Dark)
          ↓
window.matchMedia('(prefers-color-scheme: dark)') fires
          ↓
next-themes detects change (if theme === 'system')
          ↓
resolvedTheme updates to "dark"
          ↓
Same flow as manual theme change
```

### App Launch Flow

```
1. HTML loads
2. FOUC prevention script runs
   - Reads localStorage
   - Resolves "system" theme
   - Applies class immediately
3. React app mounts
4. ThemeProvider initializes
5. Syncs with existing class
6. TauriThemeSync updates window
```

## Platform Support

| Platform | Content Theme | Titlebar Theme | System Detection |
|----------|---------------|----------------|------------------|
| macOS    | ✅ Full       | ✅ Full        | ✅ Full          |
| Windows  | ✅ Full       | ✅ Full        | ✅ Full          |
| Linux    | ✅ Full       | ⚠️ Limited*    | ✅ Full          |

*Linux titlebar theming depends on desktop environment (GNOME, KDE, etc.)

## Best Practices

1. **Always use theme-aware classes**: Use `dark:` variants for all color utilities
2. **Use semantic colors**: Prefer `bg-background` over `bg-white`
3. **Test both themes**: Every UI change should be tested in light and dark mode
4. **Avoid hardcoded colors**: Use CSS variables or Tailwind theme colors
5. **Consider contrast**: Ensure sufficient contrast in both themes for accessibility

## References

- [shadcn/ui Dark Mode (Vite)](https://ui.shadcn.com/docs/dark-mode/vite) - Official implementation guide
- [Tailwind CSS v4 Dark Mode](https://tailwindcss.com/docs/dark-mode) - Official docs
- [shadcn/ui Theming](https://ui.shadcn.com/docs/theming) - Component theming guide
- [Tauri Window API](https://v2.tauri.app/reference/javascript/api/namespacewindow/) - Native window control
- [React Context API](https://react.dev/reference/react/createContext) - React documentation

## Changelog

### v1.0.0 (Current)
- Initial dark mode implementation
- Tailwind CSS v4 support
- next-themes integration
- Tauri window synchronization
- FOUC prevention
- Debug panel for development
