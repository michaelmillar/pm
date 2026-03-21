export interface Project {
  id: number;
  name: string;
  state: string;
  impact: number;
  monetization: number;
  readiness: number;
  uniqueness: number | null;
  cloneability: number | null;
  defensibility: number;
  project_type: string;
  priority_score: number;
  days_stale: number;
  last_activity: string;
  created_at: string;
  soft_deadline: string | null;
  path: string | null;
  next_milestone: string | null;
}

export interface ProjectDetail extends Project {
  inbox_note: string | null;
  roadmap: Roadmap | null;
  dod: Dod | null;
  research: Research | null;
  tasks: Task[];
}

export interface Roadmap {
  project: string;
  assessment: Assessment | null;
  phases: Phase[];
  readiness: number;
  weight_valid: boolean;
}

export interface Assessment {
  impact: number;
  monetization: number;
  cloneability: number | null;
  uniqueness: number | null;
  defensibility: number | null;
  researched_at: string;
  reasoning: string | null;
  signals: string[] | null;
  stale: boolean;
}

export interface Phase {
  id: string;
  label: string;
  weight: number;
  component: string | null;
  tasks: RoadmapTask[];
  progress: number;
}

export interface RoadmapTask {
  id: string;
  label: string;
  done: boolean;
}

export interface Dod {
  project_name: string;
  usp: string;
  criteria: Criterion[];
  complete: number;
  total: number;
}

export interface Criterion {
  id: string;
  description: string;
  evidence: string | null;
  scenario: string;
  automated: string;
  human: string;
}

export interface Research {
  summary: string;
  previous: string | null;
  researched_at: string | null;
  consecutive_flags: number;
}

export interface Task {
  plan_file: string;
  task_number: number;
  description: string;
  source: string;
}

export interface NextRecommendation {
  project: Project | null;
  reason: string;
}

export interface PipelineProject {
  id: number;
  name: string;
  project_type: string;
  readiness: number;
  priority_score: number;
  milestones: PipelineMilestone[];
}

export interface PipelineMilestone {
  name: string;
  progress: number;
  target: string | null;
}
