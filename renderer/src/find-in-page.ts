interface PinnedSearch {
  id: string;
  pattern: string;
  color: string;
  caseSensitive: boolean;
  disabled?: boolean;
}

interface MatchInfo {
  index: number;
  text: string;
  context: string;
  pattern: string;
  color: string;
}

interface SearchState {
  query: string;
  currentIndex: number;
  highlightElements: HTMLElement[];
  pinned: PinnedSearch[];
}

const state: SearchState = {
  query: "",
  currentIndex: 0,
  highlightElements: [],
  pinned: [],
};

type SearchCallback = (data: { count: number; current: number; matches: MatchInfo[] }) => void;

let callback: SearchCallback | null = null;

function highlightMatches(container: HTMLElement, query: string): number {
  // Clear existing highlights first
  clearHighlights();

  if (!query || query.length === 0) {
    return 0;
  }

  const textNodes: Text[] = [];
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => {
      const parent = node.parentElement;
      // Exclude code blocks (pre), mermaid diagrams, and already highlighted text
      // Note: inline <code> tags are intentionally searchable
      if (parent?.closest("pre, .mermaid, .search-highlight")) {
        return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });

  let node: Node | null;
  while ((node = walker.nextNode())) {
    textNodes.push(node as Text);
  }

  const queryLower = query.toLowerCase();
  state.highlightElements = [];

  // Process each text node
  for (const textNode of textNodes) {
    const text = textNode.textContent || "";
    const textLower = text.toLowerCase();
    let startIndex = 0;

    // Find all matches in this text node
    const matches: { start: number; end: number }[] = [];
    while (true) {
      const index = textLower.indexOf(queryLower, startIndex);
      if (index === -1) break;
      matches.push({ start: index, end: index + query.length });
      startIndex = index + 1;
    }

    if (matches.length === 0) continue;

    // Replace text node with highlighted fragments
    const parent = textNode.parentNode;
    if (!parent) continue;

    const fragment = document.createDocumentFragment();
    let lastEnd = 0;

    for (const match of matches) {
      // Text before match
      if (match.start > lastEnd) {
        fragment.appendChild(document.createTextNode(text.slice(lastEnd, match.start)));
      }

      // Highlighted match
      const mark = document.createElement("mark");
      mark.className = "search-highlight";
      mark.textContent = text.slice(match.start, match.end);
      fragment.appendChild(mark);
      state.highlightElements.push(mark);

      lastEnd = match.end;
    }

    // Text after last match
    if (lastEnd < text.length) {
      fragment.appendChild(document.createTextNode(text.slice(lastEnd)));
    }

    parent.replaceChild(fragment, textNode);
  }

  return state.highlightElements.length;
}

function clearHighlights(): void {
  // Remove all highlight marks and restore original text
  for (const mark of state.highlightElements) {
    const parent = mark.parentNode;
    if (parent) {
      // Replace mark with its text content
      const textNode = document.createTextNode(mark.textContent || "");
      parent.replaceChild(textNode, mark);
      // Normalize to merge adjacent text nodes
      parent.normalize();
    }
  }
  state.highlightElements = [];
  state.currentIndex = 0;
}

function navigateToMatch(direction: "next" | "prev"): number {
  if (state.highlightElements.length === 0) return 0;

  // Remove active class from current
  const current = state.highlightElements[state.currentIndex];
  current?.classList.remove("search-highlight-active");

  // Calculate new index
  if (direction === "next") {
    state.currentIndex = (state.currentIndex + 1) % state.highlightElements.length;
  } else {
    state.currentIndex =
      (state.currentIndex - 1 + state.highlightElements.length) % state.highlightElements.length;
  }

  // Add active class to new current and scroll into view
  const next = state.highlightElements[state.currentIndex];
  next?.classList.add("search-highlight-active");
  next?.scrollIntoView({ behavior: "smooth", block: "center" });

  return state.currentIndex + 1; // 1-based for display
}

export function find(query: string): void {
  state.query = query;
  const container = document.querySelector(".markdown-body");
  if (!container) {
    callback?.({ count: 0, current: 0, matches: [] });
    return;
  }

  const count = highlightMatches(container as HTMLElement, query);
  state.currentIndex = count > 0 ? 0 : -1;

  // Activate first match (no auto-scroll to avoid focus issues with IME)
  if (count > 0) {
    state.highlightElements[0]?.classList.add("search-highlight-active");
  }

  // Get all matches including pinned
  const matches = internalGetAllMatches();
  callback?.({ count, current: count > 0 ? 1 : 0, matches });
}

export function navigate(direction: "next" | "prev"): void {
  const current = navigateToMatch(direction);
  const matches = internalGetAllMatches();
  callback?.({ count: state.highlightElements.length, current, matches });
}

export function clear(): void {
  clearHighlights();
  const matches = internalGetAllMatches();
  callback?.({ count: 0, current: 0, matches });
}

export function setup(cb: SearchCallback): void {
  callback = cb;
}

// ============================================================================
// Pinned Search API
// ============================================================================

/**
 * Set pinned searches (called from Rust when state changes)
 */
export function setPinned(pinned: PinnedSearch[]): void {
  console.log("setPinned() called, pinned count:", pinned.length, "pinned:", pinned);
  state.pinned = pinned;
}

/**
 * Get current search query
 */
export function getQuery(): string {
  console.log("getQuery() called, state.query:", state.query);
  return state.query || "";
}

/**
 * Re-apply all highlights (Search + Pinned)
 * Called when tab changes or content reloads
 */
export function reapply(): void {
  console.log("reapply() called, pinned count:", state.pinned.length);
  const container = document.querySelector(".markdown-body");
  if (!container) return;

  // Clear all existing highlights
  clearAllHighlights(container as HTMLElement);

  // 1. Apply pinned searches (each with its own color)
  applyPinnedHighlights(container as HTMLElement);

  // 2. Apply active search query (yellow, navigable)
  if (state.query) {
    const count = highlightMatches(container as HTMLElement, state.query);
    state.currentIndex = count > 0 ? 0 : -1;
    if (count > 0) {
      state.highlightElements[0]?.classList.add("search-highlight-active");
    }
    const matches = internalGetAllMatches();
    callback?.({ count, current: count > 0 ? 1 : 0, matches });
  } else {
    // No active search, but still send pinned matches
    const matches = internalGetAllMatches();
    callback?.({ count: 0, current: 0, matches });
  }
}

/**
 * Clear all highlights (both search and pinned)
 */
function clearAllHighlights(container: HTMLElement): void {
  // Clear search highlights
  clearHighlights();

  // Clear pinned highlights
  const pinnedMarks = container.querySelectorAll(".pinned-highlight");
  pinnedMarks.forEach((mark) => {
    const parent = mark.parentNode;
    if (parent) {
      const textNode = document.createTextNode(mark.textContent || "");
      parent.replaceChild(textNode, mark);
      parent.normalize();
    }
  });
}

/**
 * Apply pinned search highlights (each with its own color)
 */
function applyPinnedHighlights(container: HTMLElement): void {
  for (const pinned of state.pinned) {
    // Skip disabled pinned searches
    if (pinned.disabled) continue;

    applyPinnedHighlight(container, pinned);
  }
}

/**
 * Apply a single pinned search highlight
 */
function applyPinnedHighlight(container: HTMLElement, pinned: PinnedSearch): void {
  const textNodes: Text[] = [];
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => {
      const parent = node.parentElement;
      // Exclude code blocks, mermaid diagrams, and already highlighted text
      if (parent?.closest("pre, .mermaid, .search-highlight, .pinned-highlight")) {
        return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });

  let node: Node | null;
  while ((node = walker.nextNode())) {
    textNodes.push(node as Text);
  }

  const query = pinned.pattern;
  const queryLower = pinned.caseSensitive ? query : query.toLowerCase();

  // Process each text node
  for (const textNode of textNodes) {
    const text = textNode.textContent || "";
    const textToSearch = pinned.caseSensitive ? text : text.toLowerCase();
    let startIndex = 0;

    // Find all matches in this text node
    const matches: { start: number; end: number }[] = [];
    while (true) {
      const index = textToSearch.indexOf(queryLower, startIndex);
      if (index === -1) break;
      matches.push({ start: index, end: index + query.length });
      startIndex = index + 1;
    }

    if (matches.length === 0) continue;

    // Replace text node with highlighted fragments
    const parent = textNode.parentNode;
    if (!parent) continue;

    const fragment = document.createDocumentFragment();
    let lastEnd = 0;

    for (const match of matches) {
      // Text before match
      if (match.start > lastEnd) {
        fragment.appendChild(document.createTextNode(text.slice(lastEnd, match.start)));
      }

      // Highlighted match
      const mark = document.createElement("mark");
      mark.className = "pinned-highlight";
      mark.setAttribute("data-color", pinned.color);
      mark.textContent = text.slice(match.start, match.end);
      fragment.appendChild(mark);

      lastEnd = match.end;
    }

    // Text after last match
    if (lastEnd < text.length) {
      fragment.appendChild(document.createTextNode(text.slice(lastEnd)));
    }

    parent.replaceChild(fragment, textNode);
  }
}

/**
 * Internal function to get all matches (without logs)
 */
function internalGetAllMatches(): MatchInfo[] {
  console.log("internalGetAllMatches() called");
  const matches: MatchInfo[] = [];
  const container = document.querySelector(".markdown-body");
  if (!container) {
    console.log("No container found");
    return matches;
  }

  // Helper to extract context around a match element
  function getContext(element: HTMLElement, maxLength: number = 100): string {
    const parent = element.parentElement;
    if (!parent) return element.textContent || "";

    const fullText = parent.textContent || "";
    const elementText = element.textContent || "";
    const startIndex = fullText.indexOf(elementText);

    if (startIndex === -1) return elementText;

    const contextStart = Math.max(0, startIndex - maxLength / 2);
    const contextEnd = Math.min(fullText.length, startIndex + elementText.length + maxLength / 2);

    let context = fullText.slice(contextStart, contextEnd);
    if (contextStart > 0) context = "..." + context;
    if (contextEnd < fullText.length) context = context + "...";

    return context;
  }

  // 1. Collect active search matches (yellow, navigable)
  console.log("Active search:", state.query, "highlights:", state.highlightElements.length);
  state.highlightElements.forEach((element, index) => {
    matches.push({
      index,
      text: element.textContent || "",
      context: getContext(element),
      pattern: state.query,
      color: "yellow",
    });
  });

  // 2. Collect pinned search matches (each with its own color)
  console.log("Pinned searches:", state.pinned.length);
  state.pinned.forEach((pinned) => {
    if (pinned.disabled) {
      console.log(`Skipping disabled pinned: ${pinned.pattern}`);
      return;
    }

    const pinnedMarks = container.querySelectorAll(
      `.pinned-highlight[data-color="${pinned.color}"]`
    );
    console.log(`Pinned "${pinned.pattern}" (${pinned.color}): ${pinnedMarks.length} marks`);

    pinnedMarks.forEach((mark, index) => {
      if (mark instanceof HTMLElement) {
        matches.push({
          index,
          text: mark.textContent || "",
          context: getContext(mark),
          pattern: pinned.pattern,
          color: pinned.color,
        });
      }
    });
  });

  console.log("Total matches collected:", matches.length, matches);
  return matches;
}

/**
 * Get all match information (active search + pinned searches)
 * Returns array of match details for display in sidebar
 * This is exposed for external use (e.g., manual queries)
 */
export function getAllMatches(): MatchInfo[] {
  return internalGetAllMatches();
}

/**
 * Scroll to a specific match by pattern and index
 */
export function scrollToMatch(pattern: string, index: number): void {
  const container = document.querySelector(".markdown-body");
  if (!container) return;

  let targetElement: HTMLElement | null = null;

  // Check if it's the active search
  if (pattern === state.query && index >= 0 && index < state.highlightElements.length) {
    targetElement = state.highlightElements[index];

    // Update current index and highlight
    const current = state.highlightElements[state.currentIndex];
    current?.classList.remove("search-highlight-active");

    state.currentIndex = index;
    targetElement.classList.add("search-highlight-active");
    callback?.({ count: state.highlightElements.length, current: index + 1 });
  } else {
    // Check pinned searches
    const pinned = state.pinned.find((p) => p.pattern === pattern);
    if (!pinned) return;

    const pinnedMarks = Array.from(
      container.querySelectorAll(`.pinned-highlight[data-color="${pinned.color}"]`)
    );

    if (index >= 0 && index < pinnedMarks.length) {
      targetElement = pinnedMarks[index] as HTMLElement;
    }
  }

  // Scroll to the target element
  if (targetElement) {
    targetElement.scrollIntoView({ behavior: "smooth", block: "center" });
  }
}
