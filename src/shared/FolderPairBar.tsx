import type { CompareMode } from "./compareMode";
import { folderCaption, folderRoleLabel } from "./folderLabel";

type Props = {
  mode: CompareMode;
  destinationPath: string;
  sourcePath: string;
  compact?: boolean;
};

/** Always-visible destination vs source folder names for Review / Apply / Result. */
export function FolderPairBar({
  mode,
  destinationPath,
  sourcePath,
  compact = false,
}: Props) {
  const dest = folderCaption(destinationPath);
  const source = folderCaption(sourcePath);

  return (
    <div
      className={`folder-pair-bar${compact ? " compact" : ""}`}
      role="group"
      aria-label="Folders being compared"
    >
      <div className="folder-pair-side destination">
        <p className="eyebrow">{folderRoleLabel("destination", mode)}</p>
        <p className="folder-pair-name" title={dest.full || undefined}>
          {dest.name}
        </p>
        {dest.pathHint ? (
          <p className="folder-pair-path muted tiny" title={dest.full}>
            {dest.pathHint}
          </p>
        ) : null}
      </div>
      <div className="folder-pair-sep" aria-hidden="true">
        ←
      </div>
      <div className="folder-pair-side source">
        <p className="eyebrow">{folderRoleLabel("source", mode)}</p>
        <p className="folder-pair-name" title={source.full || undefined}>
          {source.name}
        </p>
        {source.pathHint ? (
          <p className="folder-pair-path muted tiny" title={source.full}>
            {source.pathHint}
          </p>
        ) : null}
      </div>
    </div>
  );
}
