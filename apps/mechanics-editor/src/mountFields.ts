import { field, type GameplayField } from "./fieldSchema";

export const mounts = (soldier: boolean): GameplayField[] => [
  field(
    "mounts",
    "Weapon mounts",
    "Weapons",
    "Physical weapons, each an id and the label its card shows. Adding a mount gives it its own firing cycle and ammunition. Inherited mounts cannot be removed; extend a common parent to omit one.",
    { kind: "mounts" },
  ),
  field(
    "mounts.*.weapons",
    "Ammunition / weapon rows",
    "Weapons",
    "Shared weapon rows available to this physical mount. Changing the assignment changes this unit; changing a linked weapon row affects all its users.",
    { kind: "strings", references: "weapons" },
  ),
  ...(soldier
    ? [
        field(
          "mounts.*.squad",
          "Default squad gun",
          "Weapons",
          "Each living carrier fires independently; these guns share the squad targeting and ammunition readout.",
          { kind: "boolean" },
        ),
        field(
          "mounts.*.special",
          "Transferable special weapon",
          "Weapons",
          "A surviving soldier can recover this weapon and its remaining rounds when its original carrier falls.",
          { kind: "boolean" },
        ),
      ]
    : [
        field(
          "mounts.*.turret",
          "Traversing mount",
          "Weapons",
          "Turns toward its target at the turret rate and fires only within the permitted bearing tolerance.",
          { kind: "boolean" },
        ),
        field(
          "mounts.*.on",
          "Carrier mount",
          "Weapons",
          "An earlier turret mount that carries this weapon. None means attached to the hull.",
          { kind: "reference", references: "mounts", optional: true },
        ),
        field(
          "mounts.*.pivot_m",
          "Pivot position",
          "Weapons",
          "Forward, left and up coordinates in the carrier frame where this mount rotates. Geometry affects flight origins and must agree with the model.",
          { kind: "vector", unit: "m", min: undefined },
        ),
        field(
          "mounts.*.muzzle_m",
          "Muzzle offset",
          "Weapons",
          "Forward, left and up from the pivot, rotated with the mount. None is a hand weapon muzzle. Model-fit checks still apply.",
          { kind: "vector", unit: "m", min: undefined, optional: true },
        ),
      ]),
];
