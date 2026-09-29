/** "in 5 min", "in 3 h", "in 4 days", "in 2 mo", for the next-review hint. */
export function formatInterval(ms: number): string {
  const minutes = Math.round(ms / 60_000);
  if (minutes < 1) return "in a moment";
  if (minutes < 60) return `in ${minutes} min`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `in ${hours} h`;
  const days = Math.round(hours / 24);
  if (days === 1) return "tomorrow";
  if (days < 45) return `in ${days} days`;
  const months = Math.round(days / 30);
  if (months < 18) return `in ${months} mo`;
  return `in ${Math.round(days / 365)} years`;
}
