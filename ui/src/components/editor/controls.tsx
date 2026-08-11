import { type ReactNode, useEffect, useState } from "react";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";

export function Field({
  label,
  children,
}: {
  label: string;
  children: ReactNode;
}) {
  return (
    <div className="space-y-1">
      <p className="text-xs text-muted-foreground">{label}</p>
      {children}
    </div>
  );
}

/** Picks a name to start customizing; resets to the placeholder after each pick. */
export function AddEntry({
  placeholder,
  options,
  onAdd,
}: {
  placeholder: string;
  options: string[];
  onAdd: (name: string) => void;
}) {
  if (options.length === 0) return null;
  return (
    <Select value="" onValueChange={onAdd}>
      <SelectTrigger size="sm" className="w-64">
        <SelectValue placeholder={placeholder} />
      </SelectTrigger>
      <SelectContent>
        {options.map((name) => (
          <SelectItem key={name} value={name}>
            {name}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

/**
 * Text that is parsed on the way out (comma lists, hook agents) — held raw
 * while typing so a half-written entry is not rewritten under the cursor.
 */
export function CommitInput({
  label,
  value,
  placeholder,
  onCommit,
}: {
  label: string;
  value: string;
  placeholder?: string;
  onCommit: (text: string) => void;
}) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  return (
    <Input
      aria-label={label}
      value={text}
      placeholder={placeholder}
      onChange={(event) => setText(event.target.value)}
      onBlur={() => onCommit(text)}
    />
  );
}

const TRI_STATE = { true: true, false: false } as const;

export function TriStateSelect({
  label,
  value,
  onChange,
}: {
  label: string;
  value: boolean | null;
  onChange: (value: boolean | null) => void;
}) {
  return (
    <Select
      value={value === null ? "unset" : String(value)}
      onValueChange={(next) =>
        onChange(next === "unset" ? null : TRI_STATE[next as "true" | "false"])
      }
    >
      <SelectTrigger size="sm" className="w-full" aria-label={label}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectItem value="unset">unset</SelectItem>
        <SelectItem value="true">true</SelectItem>
        <SelectItem value="false">false</SelectItem>
      </SelectContent>
    </Select>
  );
}
