import { Injectable } from '@angular/core';

@Injectable({
  providedIn: 'root'
})
export class HashHoverFeature {
  hovered: boolean = false;

  constructor() {}

  showHash(): void {
    this.hovered = true;
  }

  hideHash(): void {
    this.hovered = false;
  }

  isHovered(): boolean {
    return this.hovered;
  }
}