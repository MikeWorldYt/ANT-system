import { Component, ElementRef, HostListener, QueryList, ViewChildren, OnInit, AfterViewInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';

import { IntersectionService } from '../../../services/IntersectionObserver.service';
import { LanguageService } from '../../../services/lenguaje.service';
import { Language } from '../../../services/language.types';

import { content } from '../../content/content';
import { LightboxComponent } from '../../../features/lightbox/lightbox.component';

import { HeaderT02Component } from '../header-t02/header-t02.component';

@Component({
  selector: 'doc-t02-mod-01-levels-structure',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule,
    RouterLink,
    LightboxComponent,
  ],
  templateUrl: './mod-01-levels-structure.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod01_Component implements OnInit, AfterViewInit {
  constructor(
    private intersectionService: IntersectionService,
    private languageService: LanguageService
  ) { }

  // Intersection Section <section>
  idActive: string = '';

  // For inner content
  write: any;

  // Language
  currentLanguage: Language = 'ES';

  ngOnInit(): void {
    // Initial content
    this.write = content[this.currentLanguage].title_02.module_01;

    // Suscribe to Language Service
    this.languageService.language$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.write = content[language].title_02.module_01;
        this.currentLanguage = language;
      }
    });
  }

  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
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
  
  

  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });
    this.intersectionService.getCurrentId().subscribe(id => {
      this.idActive = id;
    })

  }

}