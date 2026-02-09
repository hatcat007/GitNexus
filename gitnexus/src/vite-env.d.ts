/// <reference types="vite/client" />

// File System Access API declarations
interface FileSystemHandle {
  kind: 'file' | 'directory';
  name: string;
}

interface FileSystemFileHandle extends FileSystemHandle {
  kind: 'file';
  getFile(): Promise<File>;
}

interface FileSystemDirectoryHandle extends FileSystemHandle {
  kind: 'directory';
  entries(): AsyncIterableIterator<[string, FileSystemHandle]>;
  getDirectoryHandle(name: string, options?: { create?: boolean }): Promise<FileSystemDirectoryHandle>;
  getFileHandle(name: string, options?: { create?: boolean }): Promise<FileSystemFileHandle>;
  removeEntry(name: string, options?: { recursive?: boolean }): Promise<void>;
  resolve(possibleDescendant: FileSystemHandle): Promise<string[] | null>;
}

interface FileSystemEntry {
  name: string;
  isDirectory: boolean;
  isFile: boolean;
}

interface FileSystemFileEntry extends FileSystemEntry {
  isFile: true;
  file(successCallback: (file: File) => void, errorCallback?: (error: Error) => void): void;
}

interface FileSystemDirectoryEntry extends FileSystemEntry {
  isDirectory: true;
  createReader(): FileSystemDirectoryReader;
}

interface FileSystemDirectoryReader {
  readEntries(successCallback: (entries: FileSystemEntry[]) => void, errorCallback?: (error: Error) => void): void;
}

interface DataTransferItem {
  webkitGetAsEntry?(): FileSystemEntry | null;
}

interface Window {
  showDirectoryPicker(): Promise<FileSystemDirectoryHandle>;
  showOpenFilePicker(options?: {
    multiple?: boolean;
    excludeAcceptAllOption?: boolean;
    types?: Array<{
      description?: string;
      accept: Record<string, string[]>;
    }>;
  }): Promise<FileSystemFileHandle[]>;
  showSaveFilePicker(options?: {
    suggestedName?: string;
    excludeAcceptAllOption?: boolean;
    types?: Array<{
      description?: string;
      accept: Record<string, string[]>;
    }>;
  }): Promise<FileSystemFileHandle>;
}
