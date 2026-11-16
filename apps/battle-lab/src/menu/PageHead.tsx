import type { Ref, ReactNode } from "react";
import { hudIcon } from "@packages/scene-assets/src/icons";
import { Icon } from "@web/battle/present/icons";

/** A page's head: Back at its top left and the page's name beside it. */
export function PageHead({
  title,
  onBack,
  backRef,
  children,
}: {
  title: string;
  onBack: () => void;
  backRef?: Ref<HTMLButtonElement>;
  children?: ReactNode;
}) {
  return (
    <header className="menu-page-head">
      <button ref={backRef} type="button" className="menu-back" aria-label="Back" onClick={onBack}>
        <Icon path={hudIcon("back")} />
      </button>
      <h2>{title}</h2>
      {children}
    </header>
  );
}
