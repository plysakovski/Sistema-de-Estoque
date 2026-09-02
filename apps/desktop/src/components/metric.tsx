import type { ComponentType, SVGProps } from "react";

type Icon = ComponentType<SVGProps<SVGSVGElement>>;

interface MetricProps {
  label: string;
  value: number;
  icon: Icon;
  tone?: "default" | "warning";
}
export function Metric({ label, value, icon: IconComponent, tone = "default" }: MetricProps) {
  return (
    <div className={`metric metric--${tone}`}>
      <IconComponent aria-hidden="true" />
      <div>
        <span>{label}</span>
        <strong>{value}</strong>
      </div>
    </div>
  );
}
