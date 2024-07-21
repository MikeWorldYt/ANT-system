import { Component, ElementRef, HostListener, QueryList, ViewChildren, OnInit } from '@angular/core';
import { HeaderT02Component } from '../header-t02/header-t02.component';
import { CommonModule } from '@angular/common';
import { IntersectionService } from '../../../services/IntersectionObserver.service';
import { LightboxComponent } from '../../../features/lightbox/lightbox.component';
import { LenguageService } from '../../../services/lenguaje.service';
import { content } from '../../content/content';
// import { en } from '../../content/content.json'

@Component({
  selector: 'doc-t02-mod-01-levels-structure',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule,

    LightboxComponent,
  ],
  templateUrl: './mod-01-levels-structure.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod01_Component {
  constructor(private intersectionService: IntersectionService) { }

  // For inner content
  write: any;

  ngOnInit(): void {
    this.write = content.EN.title_02.module_01; 
  }

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