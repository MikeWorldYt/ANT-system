import { AfterViewInit, Component, ElementRef, OnInit, QueryList, ViewChildren } from '@angular/core';
import { HeaderT02Component } from '../header-t02/header-t02.component';
import { IntersectionService } from '../../../services/IntersectionObserver.service';
import { CommonModule } from '@angular/common';
import { LanguageService } from '../../../services/lenguaje.service';
import { content } from '../../content/content';
import { Language } from '../../../services/language.types';

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
export class Docs_T02_Mod02_Component implements OnInit, AfterViewInit {
  constructor(
    private intersectionService: IntersectionService,
    private languageService: LanguageService
  ) { }
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

  switchLanguage(language: Language) {
    this.languageService.setLanguage(language);
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
