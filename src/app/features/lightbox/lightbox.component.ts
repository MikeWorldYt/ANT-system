import { CommonModule } from '@angular/common';
import { Component, Input } from '@angular/core';

@Component({
  selector: 'lightbox',
  standalone: true,
  imports: [ CommonModule ],
  templateUrl: './lightbox.component.html',
  styleUrl: './lightbox.component.css'
})
export class LightboxComponent {
  @Input() src!: string;
  isLightboxOpen = false;
  lightboxImage: string | null = null;
  zoomLevel = 1;
  isDragging = false;
  startX = 0;
  startY = 0;
  translateX = 0;
  translateY = 0

  openLightbox(): void {
    this.lightboxImage = this.src;
    this.isLightboxOpen = true;
  }

  closeLightbox(): void {
    this.isLightboxOpen = false;
    this.lightboxImage = null;
    this.zoomLevel = 1;
    this.translateX = 0;
    this.translateY = 0;
    this.isDragging = false;
  }

  zoomImage(event: WheelEvent): void {
    event.preventDefault();
    const imgElement = event.target as HTMLImageElement;
    if (event.deltaY < 0) {
      // Zoom in
      this.zoomLevel += 0.1;
    } else {
      // Zoom out
      this.zoomLevel -= 0.1;
      if (this.zoomLevel < 1) {
        this.zoomLevel = 1;
      }
    }
    imgElement.style.transform = `scale(${this.zoomLevel})
      translate(${this.translateX}px, ${this.translateY}px)`;
  }

  startDrag(event: MouseEvent): void {
    event.preventDefault();
    this.isDragging = true;
    this.startX = event.clientX - this.translateX;
    this.startY = event.clientY - this.translateY;
    const imgElement = event.target as HTMLImageElement;
    imgElement.classList.add('dragging');
  }

  dragImage(event: MouseEvent): void {
    if (this.isDragging) {
      this.translateX = event.clientX - this.startX;
      this.translateY = event.clientY - this.startY;
      const imgElement = event.target as HTMLImageElement;
      imgElement.style.transform = `scale(${this.zoomLevel})
        translate(${this.translateX}px, ${this.translateY}px)`;
    }
  }

  endDrag(): void {
    this.isDragging = false;
    const imgElement = document.querySelector('.lightbox img.dragging') as HTMLImageElement;
    if (imgElement) {
      imgElement.classList.remove('dragging');
    }
  }
}
