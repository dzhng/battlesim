import { useEffect, useState } from "react";
export function NumberField({
  label,
  value,
  max = 2,
  onChange,
}: {
  label: string;
  value: number;
  max?: number;
  onChange(value: number): void;
}) {
  const [text, setText] = useState(String(value));
  useEffect(() => {
    if (Number.isFinite(value)) setText(String(value));
  }, [value]);
  return (
    <label>
      {label}
      <input
        type="number"
        min="0"
        max={max}
        step="0.01"
        required
        aria-label={label}
        value={text}
        onChange={(event) => {
          setText(event.target.value);
          onChange(event.target.value === "" ? NaN : Number(event.target.value));
        }}
      />
    </label>
  );
}
