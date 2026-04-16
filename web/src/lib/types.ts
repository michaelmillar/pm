export interface Project {
  id: number;
  name: string;
  state: string;
  archetype: string;
  stage: number;
  stage_label: string;
  velocity: number | null;
  fit_signal: number | null;
  distinctness: number | null;
  leverage: number | null;
  score: number;
  action: string;
  action_target: string | null;
  days_stale: number;
  last_activity: string;
  created_at: string;
  soft_deadline: string | null;
  path: string | null;
}

export interface ProjectDetail extends Project {
  sunk_cost_days: number | null;
  pivot_count: number;
}

export interface NextRecommendation {
  project: Project | null;
  reason: string;
}
