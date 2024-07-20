import { Component, ElementRef, HostListener, QueryList, ViewChildren } from '@angular/core';
import { HeaderT02Component } from '../header-t02/header-t02.component';
import { CommonModule } from '@angular/common';
import { IntersectionService } from '../../../services/IntersectionObserver.service';

@Component({
  selector: 'doc-t02-mod-01-levels-structure',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule
  ],
  templateUrl: './mod-01-levels-structure.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod01_Component {
  // Hash Sections
  hovered = false;

  showHash(event: Event) {
    this.hovered = true;
  }
  
  hideHash(event: Event) {
    this.hovered = false;
  }

  // service Hash Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;
  
  idActive: string = '';

  constructor(private intersectionService: IntersectionService) { }
  
  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });

    setInterval(() => {
      this.idActive = this.intersectionService.getCurrentId();
      console.log(this.idActive);
    }, 100);
  }
  // Lightbox
  isLightboxOpen = false;
  lightboxImage: string | null = null;
  zoomLevel = 1;
  isDragging = false;
  startX = 0;
  startY = 0;
  translateX = 0;
  translateY = 0;

  openLightbox(event: MouseEvent): void {
    const imgElement = event.target as HTMLImageElement;
    this.lightboxImage = imgElement.src;
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