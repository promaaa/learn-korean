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
  | { type: "response"; korean: string; options: string[] }
  | { type: "build"; english: string; chunks: string[] };

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

export type Answer = { type: "choice"; index: number } | { type: "order"; order: number[] };

export interface Feedback {
  correct: boolean;
  /** Right option of a choice game, `null` in the build game. */
  correctIndex: number | null;
  /** English of each option in the reply game (empty otherwise). */
  translations: string[];
  rating: Rating;
  korean: string;
  english: string;
  note: string | null;
  streak: number;
  retry: boolean;
  /** Next scheduled review of this card, from now. */
  dueInMs: number;
  /** Experience earned by this answer. */
  xp: number;
}

export interface Progress {
  done: number;
  remaining: number;
  correct: number;
  streak: number;
  bestStreak: number;
  /** Experience earned in this session. */
  xp: number;
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
  profile: Profile;
  levelUp: LevelUp | null;
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

export interface Level {
  level: number;
  totalXp: number;
  xpInLevel: number;
  xpForNext: number;
}

export interface PackStatus {
  id: string;
  title: string;
  unlockLevel: number;
  unlocked: boolean;
  items: number;
}

export interface Profile {
  level: Level;
  dayStreak: number;
  packs: PackStatus[];
}

export interface LevelUp {
  level: number;
  unlocked: string[];
}

export function profile(): Promise<Profile> {
  return invoke("profile");
}
