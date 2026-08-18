import { Textarea } from "@/components/ui/textarea";
import { SHARED_ALSO_APPLIES, SHARED_VIEW } from "@/lib/copy-customize";
import { useNavStore } from "@/stores/nav";

/**
 * One block of instructions vstack writes alongside what the author wrote.
 * When the same table has a shared row, this says so and offers the way to
 * it — text arriving from somewhere you cannot see is the confusing part,
 * not the text itself.
 */
export function InstructionBox({
  label,
  help,
  value,
  shared,
  onChange,
}: {
  label: string;
  help: string;
  value: string | null;
  /** Text the shared `all` row adds to every package of this kind. */
  shared: string | null;
  onChange: (text: string | null) => void;
}) {
  const goTo = useNavStore((s) => s.goTo);
  // Prose keeps a reading measure even where the page around it is wide.
  return (
    <div className="flex max-w-3xl flex-col gap-2">
      <div>
        <p className="text-sm font-medium">{label}</p>
        <p className="text-[13px] text-muted-foreground">{help}</p>
      </div>
      <Textarea
        aria-label={label}
        rows={4}
        value={value ?? ""}
        onChange={(event) =>
          onChange(event.target.value === "" ? null : event.target.value)
        }
      />
      {shared ? (
        <p className="text-xs text-muted-foreground">
          {SHARED_ALSO_APPLIES}{" "}
          <button
            type="button"
            className="underline underline-offset-2 hover:text-foreground"
            onClick={() => goTo("customize")}
          >
            {SHARED_VIEW}
          </button>
        </p>
      ) : null}
    </div>
  );
}
