export type NoteInfo = {
  id: string;
  title: string;
  slug: string;
  tags: string[];
  updated_at: string;
};

export type SearchResult = {
  note_id: string;
  title: string;
  excerpt: string;
  match_kind: string;
  score: number;
};
