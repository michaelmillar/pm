import type { Project, ProjectDetail, NextRecommendation, PortfolioStats } from "./types";

const BASE = "/api";

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`);
  if (!res.ok) throw new Error(`${res.status} ${res.statusText}`);
  return res.json();
}

export const fetchProjects = (all = false) => get<Project[]>(`/projects${all ? "?all=true" : ""}`);
export const fetchProject = (id: number) => get<ProjectDetail>(`/projects/${id}`);
export const fetchArchived = () => get<Project[]>("/archived");
export const fetchNext = () => get<NextRecommendation>("/next");
export const fetchStats = () => get<PortfolioStats>("/stats");
