import { shouldIgnorePath } from '../config/ignore-service';
import { FileEntry } from './zip';

/**
 * Check if File System Access API is supported
 */
export const isFileSystemAccessSupported = (): boolean => {
  return 'showDirectoryPicker' in window;
};

/**
 * Recursively read all files from a directory handle
 * Converts FileSystemDirectoryHandle to FileEntry[] format
 */
export const readDirectoryRecursively = async (
  dirHandle: FileSystemDirectoryHandle,
  basePath = '',
  files: FileEntry[] = [],
  onProgress?: (processed: number, total: number) => void
): Promise<FileEntry[]> => {
  let processed = 0;
  
  // First pass: count total entries for progress
  let total = 0;
  const countEntries = async (handle: FileSystemDirectoryHandle, path: string) => {
    for await (const [name, entry] of handle.entries()) {
      const entryPath = path ? `${path}/${name}` : name;
      if (!shouldIgnorePath(entryPath)) {
        total++;
        if (entry.kind === 'directory') {
          await countEntries(entry as FileSystemDirectoryHandle, entryPath);
        }
      }
    }
  };
  await countEntries(dirHandle, basePath);
  
  // Second pass: read files
  const readEntries = async (handle: FileSystemDirectoryHandle, path: string) => {
    for await (const [name, entry] of handle.entries()) {
      const entryPath = path ? `${path}/${name}` : name;
      
      // Skip ignored paths
      if (shouldIgnorePath(entryPath)) {
        continue;
      }
      
      if (entry.kind === 'directory') {
        await readEntries(entry as FileSystemDirectoryHandle, entryPath);
      } else {
        try {
          const file = await (entry as FileSystemFileHandle).getFile();
          // Skip empty or binary files
          if (file.size === 0) continue;
          if (isBinaryFile(file.name)) continue;
          
          const content = await file.text();
          files.push({
            path: entryPath,
            content
          });
        } catch (err) {
          // Skip files that can't be read as text
          console.warn(`Failed to read file: ${entryPath}`, err);
        }
      }
      
      processed++;
      onProgress?.(processed, total);
    }
  };
  
  await readEntries(dirHandle, basePath);
  return files;
};

/**
 * Pick a directory using File System Access API
 * Returns FileEntry[] compatible with existing pipeline
 */
export const pickAndReadDirectory = async (
  onProgress?: (phase: string, percent: number) => void
): Promise<{ name: string; files: FileEntry[] }> => {
  if (!isFileSystemAccessSupported()) {
    throw new Error('File System Access API not supported in this browser. Use Chrome or Edge.');
  }
  
  try {
    onProgress?.('selecting', 0);
    
    // Open directory picker
    const dirHandle = await window.showDirectoryPicker();
    const folderName = dirHandle.name;
    
    onProgress?.('reading', 5);
    
    // Read all files recursively
    let lastPercent = 5;
    const files = await readDirectoryRecursively(
      dirHandle,
      '',
      [],
      (processed, total) => {
        if (total > 0) {
          const percent = 5 + Math.round((processed / total) * 95);
          if (percent !== lastPercent) {
            lastPercent = percent;
            onProgress?.('reading', percent);
          }
        }
      }
    );
    
    onProgress?.('complete', 100);
    
    return { name: folderName, files };
  } catch (err) {
    if (err instanceof Error && err.name === 'AbortError') {
      throw new Error('User cancelled folder selection');
    }
    throw err;
  }
};

/**
 * Check if a file appears to be binary based on extension
 */
const isBinaryFile = (filename: string): boolean => {
  const binaryExtensions = new Set([
    // Images
    '.png', '.jpg', '.jpeg', '.gif', '.bmp', '.svg', '.ico', '.webp', '.avif',
    // Videos
    '.mp4', '.webm', '.ogg', '.mov', '.avi', '.mkv',
    // Audio
    '.mp3', '.wav', '.flac', '.aac', '.ogg', '.wma',
    // Archives
    '.zip', '.tar', '.gz', '.bz2', '.7z', '.rar', '.xz',
    // Binaries
    '.exe', '.dll', '.so', '.dylib', '.bin', '.dmg', '.pkg', '.deb', '.rpm',
    // Fonts
    '.woff', '.woff2', '.ttf', '.otf', '.eot',
    // Documents
    '.pdf', '.doc', '.docx', '.xls', '.xlsx', '.ppt', '.pptx',
    // Other
    '.lockb', '.wasm', '.map', '.ico'
  ]);
  
  const ext = '.' + filename.split('.').pop()?.toLowerCase();
  return binaryExtensions.has(ext);
};

/**
 * Handle dropped folder using DataTransferItem.webkitGetAsEntry()
 * This works for drag-and-drop of folders
 */
export const handleDroppedFolder = async (
  item: DataTransferItem,
  onProgress?: (phase: string, percent: number) => void
): Promise<{ name: string; files: FileEntry[] }> => {
  const entry = item.webkitGetAsEntry?.();
  
  if (!entry) {
    throw new Error('Browser does not support folder drag-and-drop');
  }
  
  if (!entry.isDirectory) {
    throw new Error('Dropped item is not a folder');
  }
  
  onProgress?.('reading', 0);
  
  const files: FileEntry[] = [];
  await readEntryRecursively(entry as FileSystemDirectoryEntry, '', files, onProgress);
  
  onProgress?.('complete', 100);
  
  return { name: entry.name, files };
};

/**
 * Recursively read files from FileSystemEntry (drag-and-drop API)
 */
const readEntryRecursively = (
  entry: FileSystemEntry,
  basePath: string,
  files: FileEntry[],
  onProgress?: (phase: string, percent: number) => void
): Promise<void> => {
  return new Promise((resolve, reject) => {
    const path = basePath ? `${basePath}/${entry.name}` : entry.name;
    
    // Skip ignored paths
    if (shouldIgnorePath(path)) {
      resolve();
      return;
    }
    
    if (entry.isDirectory) {
      const dirReader = (entry as FileSystemDirectoryEntry).createReader();
      
      const readEntries = () => {
        dirReader.readEntries(async (entries) => {
          if (entries.length === 0) {
            resolve();
            return;
          }
          
          for (const subEntry of entries) {
            await readEntryRecursively(subEntry, path, files, onProgress);
          }
          
          // Continue reading (readEntries may return in batches)
          readEntries();
        }, (err) => reject(err));
      };
      
      readEntries();
    } else {
      (entry as FileSystemFileEntry).file(
        async (file) => {
          // Skip empty or binary files
          if (file.size === 0) {
            resolve();
            return;
          }
          if (isBinaryFile(file.name)) {
            resolve();
            return;
          }
          
          try {
            const content = await file.text();
            files.push({ path, content });
          } catch (err) {
            // Skip files that can't be read as text
            console.warn(`Failed to read file: ${path}`, err);
          }
          resolve();
        },
        (err) => reject(err)
      );
    }
  });
};

/**
 * Check if dropped item is a folder
 */
export const isDroppedItemFolder = (item: DataTransferItem): boolean => {
  const entry = item.webkitGetAsEntry?.();
  return entry?.isDirectory ?? false;
};
