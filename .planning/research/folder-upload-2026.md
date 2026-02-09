# Folder Upload Research — GitNexus (February 2026)

## Current Upload Methods

### 1. ZIP Upload (`src/services/zip.ts`)
- Uses `JSZip` library to extract `.zip` files client-side
- Returns `FileEntry[]` with `{ path, content }` for each file
- Strips root prefix (e.g., "repo-main/") to normalize paths
- Applies ignore rules via `shouldIgnorePath()` from `ignore-service.ts`
- Progress: fake interval-based progress (JSZip doesn't expose progress events)

### 2. GitHub Clone (`src/services/git-clone.ts`)
- Uses `isomorphic-git` + `@isomorphic-git/lightning-fs` for in-browser git clone
- Clones to IndexedDB-backed virtual filesystem
- Recursively reads files with `readAllFiles()`, skips `.git` directory
- Progress: real progress from git callbacks
- Returns same `FileEntry[]` format for compatibility

## Common Pipeline Flow

```
FileEntry[] → processStructure → processParsing → processImports → 
processCalls → processHeritage → processCommunities → processProcesses
```

Both upload methods converge at `runPipelineFromFiles()` in `pipeline.ts`.

---

## Folder Upload Options (2026 Best Practices)

### Option 1: File System Access API (RECOMMENDED)

**API:** `window.showDirectoryPicker()`

**Best for:** Modern Chromium browsers (Chrome, Edge) — primary target for GitNexus

**Key methods:**
```javascript
const dirHandle = await window.showDirectoryPicker();
for await (const [name, handle] of dirHandle.entries()) {
  if (handle.kind === 'file') {
    const file = await handle.getFile();
    const content = await file.text();
    files.push({ path: relativePath, content });
  }
}
```

**Advantages:**
- Native file picker UI (familiar to users)
- No need to zip/unzip
- Can traverse subdirectories recursively
- Handles are serializable to IndexedDB (can persist for re-open)
- Streaming-friendly for large files

**Browser Support (2026):**
- Chrome/Edge: Full support (2020+)
- Firefox: Partial (behind flag, not default)
- Safari: Limited

**Progressive Enhancement Strategy:**
```javascript
if ('showDirectoryPicker' in window) {
  // Use File System Access API
} else {
  // Fallback to webkitdirectory input
}
```

---

### Option 2: webkitdirectory Input Attribute (FALLBACK)

**HTML:** `<input type="file" webkitdirectory directory>`

**Best for:** Non-Chromium browsers (Firefox, Safari)

**Advantages:**
- Works in Firefox and Safari
- Simple implementation
- Well-supported fallback

**Limitations:**
- Non-standard attribute (though widely supported)
- UX is less polished (shows file count, not folder picker)
- Cannot drag-and-drop a folder
- No persistent handle

---

### Option 3: Drag-and-Drop DataTransferItem.webkitGetAsEntry()

**API:** `e.dataTransfer.items` + `webkitGetAsEntry()`

**Best for:** Drag-and-drop scenarios

```javascript
const items = e.dataTransfer.items;
for (let item of items) {
  const entry = item.webkitGetAsEntry();
  if (entry.isDirectory) {
    // Recursively read with readDirectoryEntry()
  }
}
```

**Advantages:**
- Natural UX (drag folder from file manager)
- No picker needed
- Works alongside ZIP drag-and-drop

**Limitations:**
- `webkitGetAsEntry()` is non-standard but widely supported
- Requires recursive traversal implementation

---

## Recommended Implementation Strategy

### Phase 1: File System Access API (Primary)
- Add "Folder Upload" tab to DropZone
- Use `showDirectoryPicker()` when available
- Recursively traverse directory with `FileSystemDirectoryHandle.entries()`
- Convert to `FileEntry[]` format for pipeline compatibility

### Phase 2: Drag-and-Drop Enhancement
- Extend existing `handleDrop()` to detect folders via `webkitGetAsEntry()`
- Support both ZIP files (current) AND folders (new) in drop zone
- Recursive read with `FileSystemDirectoryEntry` API

### Phase 3: webkitdirectory Fallback (Optional)
- Add hidden `<input webkitdirectory>` for Firefox/Safari users
- Same recursive traversal pattern

## Directory Traversal Implementation

```javascript
// Recursive directory reader for File System Access API
async function readDirectoryRecursively(dirHandle, basePath = '', files = []) {
  for await (const [name, handle] of dirHandle.entries()) {
    const path = basePath ? `${basePath}/${name}` : name;
    
    if (handle.kind === 'directory') {
      await readDirectoryRecursively(handle, path, files);
    } else {
      const file = await handle.getFile();
      if (file.size > 0) {
        const content = await file.text();
        files.push({ path, content });
      }
    }
  }
  return files;
}
```

For `webkitGetAsEntry()`:
```javascript
// Recursive reader for FileSystemEntry API (drag-and-drop)
function readEntryRecursively(entry, basePath = '', files = []) {
  return new Promise((resolve) => {
    if (entry.isDirectory) {
      const reader = entry.createReader();
      reader.readEntries(async (entries) => {
        for (const subEntry of entries) {
          const path = basePath ? `${basePath}/${subEntry.name}` : subEntry.name;
          await readEntryRecursively(subEntry, path, files);
        }
        resolve(files);
      });
    } else {
      entry.file((file) => {
        const reader = new FileReader();
        reader.onload = () => {
          files.push({ path: basePath, content: reader.result });
          resolve(files);
        };
        reader.readAsText(file);
      });
    }
  });
}
```

## Ignore Rules Application

Same pattern as ZIP extraction:
- Use `shouldIgnorePath(path)` before adding to `files[]`
- Skip binary files (detected by extension or content check)
- Skip hidden directories (starting with `.`)

## Progress Tracking

Since folder reading is synchronous/sequential:
```javascript
let processed = 0;
let totalEstimate = 0;

// Initial count pass (fast)
const countFiles = async (dirHandle) => { /* ... */ };
totalEstimate = await countFiles(dirHandle);

// Second pass with progress
for await (const [name, handle] of dirHandle.entries()) {
  // ... read file
  processed++;
  onProgress(Math.round((processed / totalEstimate) * 100));
}
```

Or use streaming approach for better perceived performance.

## Session Source Tracking

Add new source type:
```typescript
type SessionSource = 
  | { type: 'zip'; fileName: string }
  | { type: 'github'; url: string; branch: string }
  | { type: 'folder'; name: string };  // NEW
```

## Security Considerations

- File System Access API requires user gesture (click event)
- No raw paths exposed — only relative paths from selected root
- Permission is session-scoped (can request persistent with `requestPermission()`)
- Content stays client-side (no server upload)

## References

- [MDN: File System API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API) (Oct 2025)
- [fsjs.dev: Understanding File System Access API](https://fsjs.dev/understanding-file-system-access-api/)
- [File System Access spec](https://wicg.github.io/file-system-access/)
- [Caniuse: File System Access](https://caniuse.com/mdn-api_file_system_access)
