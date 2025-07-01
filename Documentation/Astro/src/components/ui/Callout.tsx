import React from "react";
import '../../styles/callout.css';

type CalloutType = 'quote' | 'info' | 'success' | 'warning' | 'danger';

interface CalloutProps {
  type: CalloutType;
  children: React.ReactNode;
}

const iconMap: Record<CalloutType, string> = {
  quote: '/call-quote.svg',
  info: '/call-info.svg',
  success: '/call-success.svg',
  warning: '/call-warning.svg',
  danger: '/call-danger.svg',
};

export function Callout({ type, children }: CalloutProps) {
  const iconSrc = iconMap[type];

  return (
    <aside className={`callout ${type}`}>
      <img src={iconSrc} alt="" />
      <div className="content">{children}</div>
    </aside>
  );
}
