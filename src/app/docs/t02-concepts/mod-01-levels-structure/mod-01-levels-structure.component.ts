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
  hovered = false;

showHash(event: Event) {
  this.hovered = true;
}

hideHash(event: Event) {
  this.hovered = false;
}

@HostListener('click', ['$even'])
onWindowScroll( $even: Event ) {
  console.log('Scrolling...');
  }

  // service 
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

}