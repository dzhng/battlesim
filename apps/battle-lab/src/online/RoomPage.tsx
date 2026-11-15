import { useLocation, useNavigate } from "react-router";
import { PageHead } from "../menu/PageHead";

/** Slice 16's landing route, replaced by the room driver in slice 17. */
export function RoomPage() {
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <>
      <PageHead title="Room lobby" onBack={() => void navigate(`/multiplayer${location.search}`)} />
      <p>Room {decodeURIComponent(location.pathname.split("/").at(-1)!)}</p>
    </>
  );
}
