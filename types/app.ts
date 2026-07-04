export type AppState = {
  current?: string | null;
  conn_st?: 'Connected' | 'Disconnected';
  pub_ip?: string | null;
  encryption_enabled?: boolean;
  is_unlocked?: boolean;
};
