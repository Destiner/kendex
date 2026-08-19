import { Button } from "@/components/ui/button";
import { TableCell, TableRow } from "@/components/ui/table";

/** The Installed table with no rows to show: a narrowed view offers to
 * clear itself, a genuinely empty library points at Marketplaces. */
export function TableEmptyRow({
  hasAnyItems,
  onClearFilters,
  onBrowse,
}: {
  hasAnyItems: boolean;
  onClearFilters: () => void;
  onBrowse: () => void;
}) {
  return (
    <TableRow>
      <TableCell colSpan={8} className="py-10">
        {hasAnyItems ? (
          <div className="flex flex-col items-center gap-3 text-center">
            <div>
              <p className="font-medium">Nothing matches</p>
              <p className="text-sm text-muted-foreground">
                Try a different search or filter.
              </p>
            </div>
            <Button variant="outline" size="sm" onClick={onClearFilters}>
              Clear filters
            </Button>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-3 text-center">
            <div>
              <p className="font-medium">Nothing installed yet</p>
              <p className="text-sm text-muted-foreground">
                Browse Marketplaces to install skills, agents and more.
              </p>
            </div>
            <Button variant="outline" size="sm" onClick={onBrowse}>
              Browse Marketplaces
            </Button>
          </div>
        )}
      </TableCell>
    </TableRow>
  );
}
