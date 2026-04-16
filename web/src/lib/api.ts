import type { Project, ProjectDetail, NextRecommendation } from "./types";

const BASE = "/api";

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

export const fetchProjects = () => get<Project[]>("/projects");
export const fetchProject = (id: number) => get<ProjectDetail>(`/projects/${id}`);
export const fetchArchived = () => get<Project[]>("/archived");
export const fetchNext = () => get<NextRecommendation>("/next");
