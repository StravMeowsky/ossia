import * as SliderPrimitive from '@radix-ui/react-slider';
import type { ComponentProps } from 'react';

import { cn } from '../../lib/utils.ts';

import './ui.css';

export function Slider({
  className,
  'aria-label': ariaLabel,
  ...props
}: ComponentProps<typeof SliderPrimitive.Root>) {
  const values = props.value ?? props.defaultValue ?? [props.min ?? 0];
  return (
    <SliderPrimitive.Root data-slot="slider" className={cn('ui-slider', className)} {...props}>
      <SliderPrimitive.Track data-slot="slider-track" className="ui-slider-track">
        <SliderPrimitive.Range data-slot="slider-range" className="ui-slider-range" />
      </SliderPrimitive.Track>
      {values.map((_, index) => (
        <SliderPrimitive.Thumb
          key={index}
          data-slot="slider-thumb"
          className="ui-slider-thumb"
          aria-label={ariaLabel}
        />
      ))}
    </SliderPrimitive.Root>
  );
}
