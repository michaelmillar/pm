import type { Project, ProjectDetail, NextRecommendation } from "./types";

const BASE = "/api";

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

export const fetchProjects = () => get<Project[]>("/projects");
export const fetchProject = (id: number) => get<ProjectDetail>(`/projects/${id}`);
export const fetchInbox = () => get<Project[]>("/inbox");
export const fetchNext = () => get<NextRecommendation>("/next");
export const fetchParked = () => get<Project[]>("/parked");
export const fetchTrash = () => get<Project[]>("/trash");
