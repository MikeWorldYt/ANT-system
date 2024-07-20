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

  openLightbox(event: MouseEvent): void {
    const imgElement = event.target as HTMLImageElement;
    this.lightboxImage = imgElement.src;
    this.isLightboxOpen = true;
  }

  closeLightbox(): void {
    this.isLightboxOpen = false;
    this.lightboxImage = null;
    this.zoomLevel = 1;
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
    imgElement.style.transform = `scale(${this.zoomLevel})`;
  }

}