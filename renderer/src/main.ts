import "../style/main.css";

import { type Theme, getSystemTheme } from "./theme";
import * as markdownViewer from "./markdown-viewer";
import * as syntaxHighlighter from "./syntax-highlighter";
import * as mermaidRenderer from "./mermaid-renderer";
import { renderCoordinator } from "./render-coordinator";
import { setup as setupContextMenu, restoreSelection } from "./context-menu-handler";
import * as findInPage from "./find-in-page";

// Declare global Arto namespace
declare global {
  interface Window {
    Arto: {
      setupContextMenu: typeof setupContextMenu;
      restoreSelection: typeof restoreSelection;
      search: {
        setup: typeof findInPage.setup;
        find: typeof findInPage.find;
        navigate: typeof findInPage.navigate;
        clear: typeof findInPage.clear;
        setPinned: typeof findInPage.setPinned;
        getQuery: typeof findInPage.getQuery;
        reapply: typeof findInPage.reapply;
        getAllMatches: typeof findInPage.getAllMatches;
        scrollToMatch: typeof findInPage.scrollToMatch;
      };
    };
  }
}

function getCurrentTheme(): Theme {
  const theme = document.body.getAttribute("data-theme");
  switch (theme) {
    case "light":
    case "dark":
      return theme;
    default:
      return getSystemTheme();
  }
}

export function setCurrentTheme(theme: Theme) {
  document.body.setAttribute("data-theme", theme);
  markdownViewer.setTheme(theme);
  syntaxHighlighter.setTheme(theme);
  mermaidRenderer.setTheme(theme);
  renderCoordinator.forceRenderMermaid();
}

export function init(): void {
  markdownViewer.mount();
  syntaxHighlighter.mount();
  mermaidRenderer.init();
  renderCoordinator.init();

  // Expose Arto API on window for Rust interop
  window.Arto = {
    setupContextMenu,
    restoreSelection,
    search: {
      setup: findInPage.setup,
      find: findInPage.find,
      navigate: findInPage.navigate,
      clear: findInPage.clear,
      setPinned: findInPage.setPinned,
      getQuery: findInPage.getQuery,
      reapply: findInPage.reapply,
      getAllMatches: findInPage.getAllMatches,
      scrollToMatch: findInPage.scrollToMatch,
    },
  };

  // Listen for theme changes from Rust
  document.addEventListener("arto:theme-changed", ((event: CustomEvent) => {
    setCurrentTheme(event.detail);
  }) as EventListener);

  // Set initial theme
  setCurrentTheme(getCurrentTheme());
}

// Re-export mermaid window functions
export { initMermaidWindow } from "./mermaid-window-controller";
