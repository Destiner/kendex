import { useState } from "react";
import type { Scope } from "@/bindings";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { scopeLabel } from "@/lib/derive";
import { scopeName } from "@/lib/labels";

/** The one form for pointing vstack at a new catalog, wherever it's opened from. */
export function AddCatalogDialog({
  open,
  onOpenChange,
  scopes,
  busy,
  onAdd,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  scopes: Scope[];
  busy: boolean;
  onAdd: (scope: Scope, name: string, reference: string) => Promise<void>;
}) {
  const [name, setName] = useState("");
  const [reference, setReference] = useState("");
  const [targetScope, setTargetScope] = useState(scopeLabel(scopes[0]));

  const submit = () => {
    const target =
      scopes.find((s) => scopeLabel(s) === targetScope) ?? scopes[0];
    if (!name.trim() || !reference.trim()) return;
    void onAdd(target, name.trim(), reference.trim()).then(() => {
      setName("");
      setReference("");
      onOpenChange(false);
    });
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add a catalog</DialogTitle>
        </DialogHeader>
        <form
          className="space-y-3"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <div className="space-y-1.5">
            <Label htmlFor="catalog-target">Add to</Label>
            <Select value={targetScope} onValueChange={setTargetScope}>
              <SelectTrigger id="catalog-target" className="w-full">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {scopes.map((scope) => (
                  <SelectItem key={scopeLabel(scope)} value={scopeLabel(scope)}>
                    {scopeName(scope)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="catalog-name">Name</Label>
            <Input
              id="catalog-name"
              placeholder="team-tools"
              value={name}
              onChange={(e) => setName(e.target.value)}
            />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="catalog-location">Location</Label>
            <Input
              id="catalog-location"
              placeholder="owner/repo on GitHub, or a folder path"
              value={reference}
              onChange={(e) => setReference(e.target.value)}
            />
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={busy}>
              Add catalog
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
