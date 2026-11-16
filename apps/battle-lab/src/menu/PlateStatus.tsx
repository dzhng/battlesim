/** What a page's plate says while it has nothing else to show: the loading
 *  screen's lit stage bar at the centre of the plate under the page's head
 *  while it waits, a hollow one when it has failed, and below it what it waits
 *  for or why it failed. Its label is its accessible name. */
export function PlateStatus({
  label,
  detail,
  failed = false,
}: {
  label: string;
  detail?: string;
  failed?: boolean;
}) {
  return (
    <div
      className="plate-status"
      role="status"
      aria-label={label}
      data-failed={failed || undefined}
    >
      <p>
        <strong>{label}</strong>
        {detail && <span>{detail}</span>}
      </p>
    </div>
  );
}
