export type Profile = {
  name: string;
  content: string;
};

export type ProfilePartial = {
  name: string;
  content: string;
};

export type ImportResult = {
  success: string[];
  failed: ImportError[];
};

export type ImportError = {
  file_name: string;
  error: string;
};

export type ExportResult = {
  success: string[];
  failed: ExportError[];
};

export type ExportError = {
  profile_name: string;
  error: string;
};
