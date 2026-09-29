import { invoke } from "@tauri-apps/api/core";

export interface AppStatus {
  version: string;
  database: string | null;
  schemaVersion: number | null;
  error: string | null;
}

export type Skill = "listening" | "response" | "build";
export type Rating = "again" | "hard" | "good" | "easy";

export interface Card {
  itemId: string;
  skill: Skill;
}

export type Prompt =
  | { type: "listening"; korean: string; options: string[] }
  | { type: "response"; korean: string; options: string[] };

export interface ExerciseView {
  card: Card;
  kind: "sentence" | "word";
  image: boolean;
  prompt: Prompt;
}

export interface ItemImage {
  sourceUrl: string;
  attribution: string;
}

export type Answer = { type: "choice"; index: number };

export interface Feedback {
  correct: boolean;
  correctIndex: number;
  /** English of each option in the reply game (empty otherwise). */
  translations: string[];
  rating: Rating;
  korean: string;
  english: string;
  note: string | null;
  streak: number;
  retry: boolean;
}

export interface Progress {
  done: number;
  remaining: number;
  correct: number;
  streak: number;
  bestStreak: number;
}

export interface SessionStarted {
  total: number;
  due: number;
  new: number;
}

export interface Current {
  exercise: ExerciseView | null;
  progress: Progress;
}

export interface Answered {
  feedback: Feedback;
  progress: Progress;
}

export function appStatus(): Promise<AppStatus> {
  return invoke("app_status");
}

export function sessionStart(): Promise<SessionStarted> {
  return invoke("session_start");
}

export function sessionCurrent(): Promise<Current> {
  return invoke("session_current");
}

export function sessionAnswer(answer: Answer, elapsedMs: number): Promise<Answered> {
  return invoke("session_answer", { answer, elapsedMs });
}

/** Speaks Korean text (synthesized in Rust, played natively). */
export function speak(text: string): Promise<void> {
  return invoke("speak", { text });
}

/** Credit for an item's photo (the picture itself is served at `kimg://localhost/<item id>`). */
export function itemImage(itemId: string): Promise<ItemImage | null> {
  return invoke("item_image", { itemId });
}
