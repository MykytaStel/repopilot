import React from 'react';

interface CardProps {
  title: string;
}

// Props are typed with a TypeScript interface, not the prop-types package.
export function Card({ title }: CardProps) {
  return <div>{title}</div>;
}
