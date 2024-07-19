import { Component, ElementRef, QueryList, ViewChildren } from '@angular/core';
import { HeaderT02Component } from '../header-t02/header-t02.component';
import { IntersectionService } from '../../../services/IntersectionObserver.service';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'doc-t02-mod-02-support-folders',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule
  ],
  templateUrl: './mod-02-support-folders.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod02_Component {
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


}
