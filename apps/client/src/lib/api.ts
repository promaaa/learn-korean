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
  /** English of each Korean word on screen, keyed by the word as displayed (`괜찮아요?`). */
  glosses: Record<string, string>;
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

/** Which cards sessions draw from: word cards only, words before the sentences using them, or all. */
export type Focus = "words" | "guided" | "all";

export interface SessionStarted {
  total: number;
  due: number;
  new: number;
  focus: Focus;
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

/** Persists the focus; it applies from the next `sessionStart`. */
export function setFocus(focus: Focus): Promise<void> {
  return invoke("set_focus", { focus });
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

export type Finger =
  | "leftPinky"
  | "leftRing"
  | "leftMiddle"
  | "leftIndex"
  | "rightIndex"
  | "rightMiddle"
  | "rightRing"
  | "rightPinky"
  | "thumb";

export interface KeyCap {
  code: string;
  row: number;
  base: string;
  shifted: string | null;
  finger: Finger;
}

export interface NextKey {
  code: string;
  shift: boolean;
  jamo: string;
  finger: Finger;
  shiftFinger: Finger | null;
}

export interface DrillSnapshot {
  target: string;
  typed: string;
  doneChars: number;
  next: NextKey | null;
  keystrokes: number;
  errors: number;
  streak: number;
  bestStreak: number;
  accuracy: number;
  cpm: number;
  wpm: number;
  finished: boolean;
  elapsedMs: number;
}

export interface TypingTarget {
  itemId: string;
  korean: string;
  english: string;
}

export interface DrillView {
  target: TypingTarget;
  snapshot: DrillSnapshot;
  position: number;
  total: number;
}

export interface Pressed {
  outcome: { correct: boolean; finished: boolean; ignored: boolean };
  view: DrillView;
}

export function typingLayout(): Promise<KeyCap[]> {
  return invoke("typing_layout");
}

/** A lesson of the Typing Gym's keyboard course. */
export interface KeyLesson {
  id: string;
  title: string;
  /** The keys the lesson's stage teaches. */
  keys: string[];
  unlocked: boolean;
  passed: boolean;
}

export function typingLessons(): Promise<KeyLesson[]> {
  return invoke("typing_lessons");
}

/** Starts a course lesson (fails when locked), or a round of words for `null`. */
export function typingStart(lesson: number | null): Promise<DrillView> {
  return invoke("typing_start", { lesson });
}

/** The unfinished round left when the screen was switched away, if any. */
export function typingCurrent(): Promise<{ lesson: number | null; view: DrillView } | null> {
  return invoke("typing_current");
}

export function typingPress(code: string, shift: boolean, atMs: number): Promise<Pressed> {
  return invoke("typing_press", { code, shift, atMs });
}

/** A course lesson's result is scored (a pass is recorded); a round of words has none. */
export type TypingNext = { type: "line"; view: DrillView } | { type: "over"; result: LessonResult | null };

export function typingNext(): Promise<TypingNext> {
  return invoke("typing_next");
}

export interface DayActivity {
  answers: number;
  correct: number;
}

export interface Heatmap {
  /** Local day number (days since 1970-01-01 in the learner's time zone) of `days[0]`. */
  firstDay: number;
  /** Weekday of `days[0]`: 0 = Monday … 6 = Sunday. */
  firstWeekday: number;
  /** Oldest first; the last entry is today. */
  days: DayActivity[];
}

/** Answers whose previous answer to the same card was on an earlier day. */
export interface Retention {
  reviews: number;
  correct: number;
  /** `null` without reviews. */
  rate: number | null;
}

export interface SkillRetention {
  skill: Skill;
  /** Last 30 days. */
  recent: Retention;
  allTime: Retention;
}

export interface Words {
  known: number;
  /** Introduced, not known yet. */
  learning: number;
  total: number;
  sentencesReady: number;
  sentences: number;
}

export interface Totals {
  answers: number;
  correct: number;
  timeMs: number;
  practiceDays: number;
  dayStreak: number;
}

export interface Stats {
  /** Today's local day number. */
  today: number;
  heatmap: Heatmap;
  /** Milliseconds per day, the last 30 days, oldest first. */
  timePerDay: number[];
  retention: SkillRetention[];
  words: Words;
  /** Cards due per day for 14 days, today (with overdue cards) first. */
  forecast: number[];
  totals: Totals;
}

export function stats(): Promise<Stats> {
  return invoke("stats");
}

/** A jamo a primer lesson introduces. */
export interface NewJamo {
  jamo: string;
  /** Hangul name: 기역, 니은, 아… */
  name: string;
  example: { korean: string; english: string };
}

export interface HangulLesson {
  id: string;
  title: string;
  jamo: NewJamo[];
  lines: number;
  unlocked: boolean;
  passed: boolean;
}

export interface LessonResult {
  /** Index of the lesson in its list (`hangulLessons()`, `typingLessons()`). */
  lesson: number;
  accuracy: number;
  keystrokes: number;
  errors: number;
  passed: boolean;
  /** Accuracy needed to pass, in percent. */
  passPercent: number;
}

export type HangulNext = { type: "line"; view: DrillView } | { type: "over"; result: LessonResult };

export function hangulLessons(): Promise<HangulLesson[]> {
  return invoke("hangul_lessons");
}

/** Starts a round of the lesson's lines; fails when the lesson is locked. */
export function hangulStart(lesson: number): Promise<DrillView> {
  return invoke("hangul_start", { lesson });
}

/** The unfinished round left when the screen was switched away, if any. */
export function hangulCurrent(): Promise<{ lesson: number; view: DrillView } | null> {
  return invoke("hangul_current");
}

export function hangulPress(code: string, shift: boolean, atMs: number): Promise<Pressed> {
  return invoke("hangul_press", { code, shift, atMs });
}

/** The next line, or the scored round (a pass is recorded) after the last one. */
export function hangulNext(): Promise<HangulNext> {
  return invoke("hangul_next");
}
