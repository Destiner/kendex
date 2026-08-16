import {
  Bug,
  Database,
  FileText,
  GitBranch,
  Layers,
  type LucideIcon,
  Package,
  Plug,
  Route,
  ScanEye,
  Search,
  ShieldCheck,
  Sparkles,
  TestTube,
  Workflow,
  Zap,
} from "lucide-react";
import type { Tag } from "@/bindings";
import { Badge } from "@/components/ui/badge";
import { TAG_LABELS } from "@/lib/labels";
import { cn } from "@/lib/utils";

// A tag says what a thing is for, so it gets a picture of that job. Tags do
// not each get a colour of their own: hue in this app means which tool an
// item belongs to, and a second colour language on the same row would make
// both harder to read. One tint for all of them, and the icon carries which.
const TAG_ICONS: Record<Tag, LucideIcon> = {
  review: ScanEye,
  testing: TestTube,
  docs: FileText,
  research: Search,
  planning: Route,
  refactoring: Layers,
  debugging: Bug,
  security: ShieldCheck,
  performance: Zap,
  git: GitBranch,
  release: Package,
  data: Database,
  ui: Sparkles,
  integration: Plug,
  automation: Workflow,
};

export function TagBadge({ tag, className }: { tag: Tag; className?: string }) {
  const Icon = TAG_ICONS[tag];
  return (
    <Badge
      className={cn(
        "border-transparent bg-muted font-normal text-muted-foreground",
        className,
      )}
    >
      <Icon className="size-3" />
      {TAG_LABELS[tag]}
    </Badge>
  );
}

/** Every tag an item carries, in the order it was given them. */
export function TagBadges({
  tags,
  className,
}: {
  tags: Tag[];
  className?: string;
}) {
  if (tags.length === 0) return null;
  return (
    <span className={cn("flex flex-wrap gap-1", className)}>
      {tags.map((tag) => (
        <TagBadge key={tag} tag={tag} />
      ))}
    </span>
  );
}
